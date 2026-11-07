//! The model sidecar. Inkling spawns `brain/` once (Bun + the Claude Agent SDK) and
//! talks to it with one JSON object per line. Each request gets an id; replies for
//! that id (streamed text, the final result, or an error) go to the callback that
//! asked. A callback hears any number of deltas and then exactly one Done or Error,
//! and nothing after a cancel. If the brain stops, every waiting callback gets an
//! error and the next request starts it again.

use anyhow::{Context as _, Result, anyhow, bail};
use gpui_kit::{App, AppContext as _, Context, Entity, Global, Task};
use serde_json::{Value, json};

use crate::store::{ModelChoice, Store, chosen_model};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub enum BrainMsg {
    Delta(String),
    Done(Value),
    Error { kind: String, message: String },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    pub email: Option<String>,
    pub plan: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Limit {
    pub status: String,
    pub resets_at: Option<i64>,
    pub utilization: Option<f64>,
    pub kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrainState {
    Starting,
    Ready,
    Failed(String),
}

type Callback = Box<dyn FnMut(BrainMsg, &mut App)>;

pub struct Brain {
    proc: Option<Proc>,
    pending: Pending<App>,
    /// Bumped on every start, so lines still in flight from a replaced process are ignored.
    generation: u64,
    next_id: u64,
    pub state: BrainState,
    pub account: Option<Account>,
    /// The models this brain's Claude Code reported, in its order; None until it says.
    pub models: Option<Vec<ModelChoice>>,
    pub limit: Option<Limit>,
    pub last_error: Option<String>,
    _reader: Option<Task<()>>,
}

/// How long the calls nobody asked for wait after a refusal that gave no reset time.
const HOLD_SECS: i64 = 300;

/// Until when (unix seconds) the calls nobody asked for wait, because the Claude plan said
/// no; 0 when nothing is held. Kept outside the Brain entity: a page asks from inside brain
/// callbacks, while the Brain is being updated, and reading the entity then panics.
static HELD_UNTIL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn hold(until: Option<i64>) {
    HELD_UNTIL.store(until.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
}

/// The Claude plan refused a call and has not reset yet: reads, rechecks and learning
/// wait instead of failing on every pause. What you ask for still goes.
pub fn holding() -> bool {
    hold_left().is_some()
}

/// How long the hold has left, while there is one.
pub fn hold_left() -> Option<std::time::Duration> {
    let left = HELD_UNTIL.load(std::sync::atomic::Ordering::Relaxed) - chrono::Utc::now().timestamp();
    (left > 0).then(|| std::time::Duration::from_secs(left as u64))
}

/// Until when to hold after the plan refused at `now`: its reset time when it is still to
/// come, or a few minutes. Nothing is held while the plan allows calls.
fn hold_until(limit: Option<&Limit>, refused: bool, now: i64) -> Option<i64> {
    if !refused {
        return None;
    }
    Some(limit.and_then(|l| l.resets_at).filter(|t| *t > now).unwrap_or(now + HOLD_SECS))
}

/// Effort levels from least to most, as the SDK names them.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// The effort to send with `model`: `effort` when the model takes it, else the nearest
/// level it does take (the lower one on a tie), and none at all for a model that takes
/// none. A model the list does not know, or one it gives no levels for, gets `effort`.
pub fn effort_for(models: &[ModelChoice], model: &str, effort: &str) -> String {
    let Some(levels) = chosen_model(models, model).and_then(|i| models[i].efforts.as_ref()) else {
        return effort.to_string();
    };
    if levels.iter().any(|l| l == effort) {
        return effort.to_string();
    }
    let rank = |e: &str| EFFORTS.iter().position(|x| *x == e);
    let Some(want) = rank(effort) else { return levels.first().cloned().unwrap_or_default() };
    levels
        .iter()
        .filter_map(|l| rank(l).map(|r| (r.abs_diff(want), r, l)))
        .min_by_key(|(distance, r, _)| (*distance, *r))
        .map(|(_, _, l)| l.clone())
        .unwrap_or_default()
}

pub struct GlobalBrain(pub Entity<Brain>);
impl Global for GlobalBrain {}

fn brain_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("INKLING_BRAIN_DIR") {
        return PathBuf::from(dir);
    }
    // Inside a bundled app the sidecar sits next to the executable's resources.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(res) = exe.parent().and_then(|p| p.parent()).map(|p| p.join("Resources/brain")) {
            if res.exists() {
                return res;
            }
        }
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/brain"))
}

fn find_bun() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("INKLING_BUN") {
        return Some(PathBuf::from(p));
    }
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        home.join(".bun/bin/bun"),
        PathBuf::from("/opt/homebrew/bin/bun"),
        PathBuf::from("/usr/local/bin/bun"),
    ];
    if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
        return Some(found);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join("bun"))
            .find(|p| p.is_file())
    })
}

impl Brain {
    pub fn global(cx: &App) -> Entity<Brain> {
        cx.global::<GlobalBrain>().0.clone()
    }

    pub fn init(cx: &mut App) {
        let brain = cx.new(|cx| {
            let mut b = Brain {
                proc: None,
                pending: Pending::new(),
                generation: 0,
                next_id: 1,
                state: BrainState::Starting,
                account: None,
                models: None,
                limit: None,
                last_error: None,
                _reader: None,
            };
            // A failure is kept in `state`; the next request tries again.
            let _ = b.start(cx);
            b
        });
        cx.set_global(GlobalBrain(brain));
    }

    fn start(&mut self, cx: &mut Context<Self>) -> Result<()> {
        self.state = BrainState::Starting;
        let launch = Launch::from_env();
        let spawned = launch.command().and_then(|cmd| Proc::spawn(cmd, &launch.log));
        let (mut proc, stdout) = match spawned {
            Ok(v) => v,
            Err(e) => {
                log::error!("brain: {e:#}");
                self.state = BrainState::Failed(format!("{e:#}"));
                return Err(e);
            }
        };
        self.generation += 1;
        let generation = self.generation;
        let failed = proc.failed.take().expect("new writer has an error receiver");
        self.proc = Some(proc);
        let (tx, rx) = async_channel::unbounded::<Option<Value>>();
        let failure_tx = tx.clone();
        std::thread::spawn(move || {
            if let Ok(error) = failed.recv() {
                log::warn!("brain: writing its input failed: {error}");
                let _ = failure_tx.send_blocking(None);
            }
        });
        std::thread::spawn(move || {
            pump(stdout, |v| tx.send_blocking(Some(v)).is_ok());
            // `None` says the output closed; nothing the brain prints can look like it.
            let _ = tx.send_blocking(None);
        });
        self._reader = Some(cx.spawn(async move |this, cx| {
            while let Ok(line) = rx.recv().await {
                if this.update(cx, |brain, cx| brain.handle(generation, line, cx)).is_err() {
                    break;
                }
            }
        }));
        if self.account.is_none() || self.models.is_none() {
            self.request_status();
        }
        Ok(())
    }

    /// The models to offer: what Claude Code reports now, else what it reported last time
    /// (kept in settings.json), else none. The flag says whether the list is live.
    pub fn model_choices(&self, cx: &App) -> (Vec<ModelChoice>, bool) {
        match &self.models {
            Some(list) => (list.clone(), true),
            None => (cx.global::<Store>().settings.models.clone(), false),
        }
    }

    /// Ask Claude Code again who is signed in and which models it offers (no model turn).
    /// Starts the brain when it is not running; starting asks on its own.
    pub fn check_status(&mut self, cx: &mut Context<Self>) {
        if self.proc.is_none() {
            let _ = self.start(cx);
            cx.notify();
        } else {
            self.request_status();
        }
    }

    fn handle(&mut self, generation: u64, line: Option<Value>, cx: &mut Context<Self>) {
        if generation != self.generation {
            return;
        }
        let Some(ev) = line else {
            // Writer failure and stdout EOF can report the same stop. Retire this
            // process before callbacks, so later events cannot replace its failure.
            self.generation += 1;
            // Its output closed: it has stopped. Everyone still waiting hears why.
            let why = self.proc.take().map(Proc::reap).unwrap_or_else(|| "the brain stopped".into());
            log::warn!("brain: {why}");
            self.state = BrainState::Failed(why.clone());
            self.pending.fail_all(&why, cx);
            cx.notify();
            return;
        };
        match parse_event(&ev) {
            Event::Ready => {
                self.state = BrainState::Ready;
                cx.notify();
            }
            Event::Delta { id, text } => self.pending.deliver(&id, BrainMsg::Delta(text), cx),
            Event::Done { id, data } => {
                self.pending.deliver(&id, BrainMsg::Done(data), cx);
                self.last_error = None;
                cx.notify();
            }
            Event::Error { id, kind, message } => {
                if kind != "cancelled" {
                    self.last_error = Some(message.clone());
                }
                // A request refused for the plan's limit, with or without a limit event.
                if kind == "rate_limit" && !holding() {
                    hold(hold_until(self.limit.as_ref(), true, chrono::Utc::now().timestamp()));
                }
                self.pending.deliver(&id, BrainMsg::Error { kind, message }, cx);
                cx.notify();
            }
            Event::Status { account, models } => {
                self.account = account;
                if let Some(models) = models {
                    log::info!(
                        "brain: Claude Code offers {}",
                        models.iter().map(|m| m.value.as_str()).collect::<Vec<_>>().join(", ")
                    );
                    // Kept for when the brain is not running, so Settings still lists them.
                    let store = cx.global_mut::<Store>();
                    if store.settings.models != models {
                        store.settings.models = models.clone();
                        store.save_settings();
                    }
                    self.models = Some(models);
                }
                cx.notify();
            }
            Event::Limit(limit) => {
                hold(hold_until(Some(&limit), limit.status == "rejected", chrono::Utc::now().timestamp()));
                self.limit = Some(limit);
                cx.notify();
            }
            Event::Unknown => {}
        }
    }

    /// Write one request, starting the brain first if it is not running.
    fn send(&mut self, value: &Value, cx: &mut Context<Self>) -> Result<()> {
        if self.proc.is_none() {
            self.start(cx)?;
        }
        let proc = self.proc.as_mut().ok_or_else(|| anyhow!("the brain is not running"))?;
        proc.send(value).context("writing to the brain")
    }

    fn request_status(&mut self) {
        let id = format!("status-{}", self.next_id);
        self.next_id += 1;
        if let Some(proc) = self.proc.as_mut() {
            if let Err(e) = proc.send(&json!({"op": "status", "id": id})) {
                log::warn!("brain status: {e:#}");
            }
        }
    }

    /// Run a task. `input` is the task object (task name plus its fields).
    pub fn run(
        &mut self,
        input: Value,
        model: &str,
        effort: &str,
        cx: &mut Context<Self>,
        callback: impl FnMut(BrainMsg, &mut App) + 'static,
    ) -> String {
        let id = format!("r{}", self.next_id);
        self.next_id += 1;
        // A level the model does not take would fail the call; send the nearest one.
        let effort = match &self.models {
            Some(list) => effort_for(list, model, effort),
            None => effort_for(&cx.global::<Store>().settings.models, model, effort),
        };
        // Settings › Model › Look things up on the web; the brain decides which tasks may.
        let web = cx.try_global::<crate::store::Store>().is_some_and(|s| s.settings.look_up);
        let req = json!({
            "op": "run",
            "id": id,
            "settings": { "model": model, "effort": effort, "web": web },
            "input": input,
        });
        let mut callback: Callback = Box::new(callback);
        match self.send(&req, cx) {
            Ok(()) => self.pending.insert(id.clone(), callback),
            Err(e) => {
                let message = format!("{e:#}");
                self.last_error = Some(message.clone());
                cx.notify();
                // Not now: the caller is usually in the middle of updating the entity the
                // callback updates, and GPUI panics on a second update of the same entity.
                // Deferred, the error also lands after the caller has stored the id.
                cx.defer(move |cx| callback(BrainMsg::Error { kind: "other".into(), message }, cx));
            }
        }
        id
    }

    pub fn cancel(&mut self, id: &str, _cx: &mut Context<Self>) {
        self.pending.remove(id);
        // A brain that is not running has nothing to cancel; don't start one to say so.
        if let Some(proc) = self.proc.as_mut() {
            if proc.send(&json!({"op": "cancel", "id": id})).is_err() {
                // A full/broken input cannot accept cancellation. Stop the process;
                // EOF will fail every outstanding request outside this update.
                let _ = proc.child.kill();
            }
        }
    }

    /// Stop the brain for good (the app is quitting). `state` says so, as it does whenever
    /// there is no process: a running brain is `Starting` or `Ready`, no brain is `Failed`.
    pub fn shutdown(&mut self) {
        self.generation += 1;
        if let Some(proc) = self.proc.take() {
            proc.stop();
            self.state = BrainState::Failed("the brain was shut down".into());
        }
    }
}

impl Drop for Brain {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ----- pieces that do not need GPUI ----------------------------------------------

/// Callbacks waiting on requests, by id. Deltas reach a callback as they come; the
/// first Done or Error ends it and drops it, so nothing reaches a callback after its
/// end, and nothing reaches one that was cancelled.
struct Pending<C: ?Sized> {
    waiting: HashMap<String, Box<dyn FnMut(BrainMsg, &mut C)>>,
}

impl<C: ?Sized> Pending<C> {
    fn new() -> Self {
        Pending { waiting: HashMap::new() }
    }

    fn insert(&mut self, id: String, callback: Box<dyn FnMut(BrainMsg, &mut C)>) {
        self.waiting.insert(id, callback);
    }

    fn remove(&mut self, id: &str) -> bool {
        self.waiting.remove(id).is_some()
    }

    fn deliver(&mut self, id: &str, msg: BrainMsg, cx: &mut C) {
        if matches!(msg, BrainMsg::Delta(_)) {
            if let Some(callback) = self.waiting.get_mut(id) {
                callback(msg, cx);
            }
        } else if let Some(mut callback) = self.waiting.remove(id) {
            callback(msg, cx);
        }
    }

    fn fail_all(&mut self, message: &str, cx: &mut C) {
        let waiting: Vec<_> = self.waiting.drain().collect();
        for (_, mut callback) in waiting {
            callback(BrainMsg::Error { kind: "other".into(), message: message.to_string() }, cx);
        }
    }
}

/// One line from the brain, read.
#[derive(Debug, PartialEq)]
enum Event {
    Ready,
    Delta { id: String, text: String },
    Done { id: String, data: Value },
    Error { id: String, kind: String, message: String },
    Status { account: Option<Account>, models: Option<Vec<ModelChoice>> },
    Limit(Limit),
    Unknown,
}

/// The status reply's `models` (the SDK's `supportedModels()`): rows without a value are
/// dropped, as are repeats. None when there is no list, or nothing usable in it.
fn parse_models(v: Option<&Value>) -> Option<Vec<ModelChoice>> {
    let mut out: Vec<ModelChoice> = vec![];
    for m in v?.as_array()? {
        let text = |key: &str| m.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(String::from);
        let Some(value) = text("value") else { continue };
        if out.iter().any(|o| o.value == value) {
            continue;
        }
        let efforts = if m.get("supportsEffort").and_then(Value::as_bool) == Some(false) {
            Some(vec![])
        } else {
            m.get("supportedEffortLevels").and_then(Value::as_array).map(|levels| {
                levels.iter().filter_map(Value::as_str).filter(|l| EFFORTS.contains(l)).map(String::from).collect()
            })
        };
        out.push(ModelChoice {
            name: text("displayName").unwrap_or_else(|| value.clone()),
            description: text("description").unwrap_or_default(),
            resolved: text("resolvedModel"),
            efforts,
            value,
        });
    }
    (!out.is_empty()).then_some(out)
}

fn parse_event(v: &Value) -> Event {
    let s = |key: &str| v.get(key).and_then(Value::as_str);
    let id = s("id").unwrap_or("").to_string();
    match s("ev").unwrap_or("") {
        "ready" => Event::Ready,
        "delta" => Event::Delta { id, text: s("text").unwrap_or("").to_string() },
        "done" => Event::Done { id, data: v.get("data").cloned().unwrap_or(Value::Null) },
        "error" => Event::Error {
            id,
            kind: s("kind").filter(|k| !k.is_empty()).unwrap_or("other").to_string(),
            message: s("message").unwrap_or("").to_string(),
        },
        "status" => Event::Status {
            account: v.pointer("/data/account").filter(|a| a.is_object()).map(|a| {
                let field = |key: &str| a.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
                Account { email: field("email"), plan: field("subscriptionType") }
            }),
            models: parse_models(v.pointer("/data/models")),
        },
        "limit" => Event::Limit(Limit {
            status: s("status").unwrap_or("").to_string(),
            resets_at: v.get("resetsAt").and_then(|t| t.as_i64().or_else(|| t.as_f64().map(|f| f as i64))),
            utilization: v.get("utilization").and_then(Value::as_f64),
            kind: s("kind").map(String::from),
        }),
        _ => Event::Unknown,
    }
}

/// Turn the brain's stdout into JSON values until it closes or `sink` says stop. A
/// line that is not JSON (or not even UTF-8) is logged and skipped; it does not end
/// the stream.
fn pump(stdout: impl Read, mut sink: impl FnMut(Value) -> bool) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                log::warn!("brain: reading its output failed: {e}");
                return;
            }
        }
        let text = String::from_utf8_lossy(&line);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(text) {
            Ok(v) => {
                if !sink(v) {
                    return;
                }
            }
            Err(e) => {
                let head: String = text.chars().take(200).collect();
                log::warn!("brain said something that is not JSON ({e}): {head}");
            }
        }
    }
}

#[cfg(test)]
fn write_line(w: &mut impl Write, value: &Value) -> Result<()> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    w.write_all(line.as_bytes())?;
    w.flush()?;
    Ok(())
}

/// Where the brain lives and how to start it.
#[derive(Clone, Debug)]
struct Launch {
    dir: PathBuf,
    /// INKLING_BRAIN_ENTRY=<file.ts> runs another entry point with Bun (a fake brain
    /// for tests); relative paths are inside `dir`.
    entry: Option<PathBuf>,
    bun: Option<PathBuf>,
    log: PathBuf,
}

const NO_BUN: &str = "Bun is not installed (looked in ~/.bun/bin, /opt/homebrew/bin, /usr/local/bin and PATH)";

impl Launch {
    fn from_env() -> Self {
        Launch {
            dir: brain_dir(),
            entry: std::env::var_os("INKLING_BRAIN_ENTRY").filter(|e| !e.is_empty()).map(PathBuf::from),
            bun: find_bun(),
            log: crate::store::Store::data_dir().join("brain.log"),
        }
    }

    fn command(&self) -> Result<Command> {
        let bun = || self.bun.clone().ok_or_else(|| anyhow!(NO_BUN));
        let compiled = self.dir.join("dist/inkling-brain");
        let mut cmd = if let Some(entry) = &self.entry {
            // relative to the brain folder, or else to where Inkling was started from
            let mut path = self.dir.join(entry);
            if !path.is_file() {
                if let Ok(cwd) = std::env::current_dir() {
                    path = cwd.join(entry);
                }
            }
            if !path.is_file() {
                bail!("INKLING_BRAIN_ENTRY is {}, which is not a file", path.display());
            }
            let mut c = Command::new(bun()?);
            c.arg("run").arg(path);
            c
        } else if compiled.is_file() {
            Command::new(compiled)
        } else if self.dir.join("src/main.ts").is_file() {
            let mut c = Command::new(bun()?);
            c.args(["run", "src/main.ts"]);
            c
        } else {
            bail!("the brain is not in {} (no src/main.ts there)", self.dir.display());
        };
        cmd.current_dir(&self.dir);
        Ok(cmd)
    }
}

/// A running brain process.
struct Proc {
    child: Child,
    writer: SyncSender<Vec<u8>>,
    failed: Option<Receiver<String>>,
    log: PathBuf,
    /// Where this run's stderr starts in the log.
    log_start: u64,
}

impl Proc {
    fn spawn(mut cmd: Command, log_path: &Path) -> Result<(Proc, ChildStdout)> {
        if let Some(dir) = log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .with_context(|| format!("opening {}", log_path.display()))?;
        let log_start = log.metadata().map(|m| m.len()).unwrap_or(0);
        let dir = cmd.get_current_dir().map(|d| d.display().to_string()).unwrap_or_else(|| ".".into());
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .spawn()
            .with_context(|| format!("starting the brain in {dir}"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("the brain has no stdout"))?;
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("the brain has no stdin"))?;
        let (writer, lines) = mpsc::sync_channel::<Vec<u8>>(32);
        let (failed, errors) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(line) = lines.recv() {
                if let Err(e) = stdin.write_all(&line).and_then(|_| stdin.flush()) {
                    let _ = failed.send(e.to_string());
                    break;
                }
            }
        });
        Ok((Proc { child, writer, failed: Some(errors), log: log_path.to_path_buf(), log_start }, stdout))
    }

    fn send(&mut self, value: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(value)?;
        // Bound queued memory, and report overload instead of blocking the UI on a pipe.
        if bytes.len() > 2 * 1024 * 1024 { bail!("this request is too large; shorten the note or conversation before trying again"); }
        bytes.push(b'\n');
        self.writer.try_send(bytes).map_err(|e| anyhow!("the brain cannot accept this request: {e}"))
    }

    /// Its output closed: make sure it is gone, and say why it stopped.
    fn reap(mut self) -> String {
        let _ = self.child.kill();
        let mut why = "the brain stopped".to_string();
        if let Ok(status) = self.child.wait() {
            if !status.success() {
                why.push_str(&format!(" ({status})"));
            }
        }
        if let Some(reason) = crash_reason(&read_from(&self.log, self.log_start)) {
            why.push_str(": ");
            why.push_str(&reason);
        }
        why
    }

    /// Ask it to stop, give it a moment to stop its runtimes, then make sure.
    fn stop(self) {
        let Proc { mut child, writer, .. } = self;
        let _ = writer.try_send(b"{\"op\":\"shutdown\"}\n".to_vec());
        drop(writer);
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// The end of `path` from `start` on (at most the last 8 KB of it).
fn read_from(path: &Path, start: u64) -> String {
    let Ok(mut f) = std::fs::File::open(path) else { return String::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let from = start.max(len.saturating_sub(8192));
    let mut buf = Vec::new();
    if f.seek(SeekFrom::Start(from)).is_ok() {
        let _ = f.read_to_end(&mut buf);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// The line in the brain's stderr that says why it died: Bun's `error: …` or a
/// `TypeError: …`-style line, not the stack, the source excerpt or the banner.
fn crash_reason(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| {
            if l.starts_with("at ") || l.starts_with("[brain]") {
                return false;
            }
            l.to_ascii_lowercase().starts_with("error:")
                || l.split_once(": ").is_some_and(|(head, _)| head.ends_with("Error") && !head.contains(' '))
        })
        .map(|l| l.chars().take(240).collect())
}

#[cfg(test)]
mod tests {
    use super::{Account, BrainMsg, Event, HOLD_SECS, Launch, Limit, Pending, Proc, crash_reason, hold_until, parse_event, pump, write_line};
    use serde_json::{Value, json};
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
        sync::mpsc,
        time::Duration,
    };

    type Log = Rc<RefCell<Vec<String>>>;

    fn recorder(name: &str, log: &Log) -> Box<dyn FnMut(BrainMsg, &mut ())> {
        let (name, log) = (name.to_string(), log.clone());
        Box::new(move |msg, _| {
            let what = match msg {
                BrainMsg::Delta(t) => format!("{name}:delta:{t}"),
                BrainMsg::Done(v) => format!("{name}:done:{v}"),
                BrainMsg::Error { kind, message } => format!("{name}:error:{kind}:{message}"),
            };
            log.borrow_mut().push(what);
        })
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("inkling-brain-rs-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[std::prelude::v1::test]
    fn each_request_ends_once_and_cancel_silences_it() {
        let log: Log = Rc::default();
        let mut p: Pending<()> = Pending::new();
        p.insert("a".into(), recorder("a", &log));
        p.insert("b".into(), recorder("b", &log));
        p.deliver("a", BrainMsg::Delta("x".into()), &mut ());
        p.deliver("a", BrainMsg::Delta("y".into()), &mut ());
        p.deliver("a", BrainMsg::Done(json!({"text": "xy"})), &mut ());
        // Late lines for a finished request go nowhere.
        p.deliver("a", BrainMsg::Error { kind: "other".into(), message: "late".into() }, &mut ());
        p.deliver("a", BrainMsg::Delta("z".into()), &mut ());
        // A cancelled request hears nothing more.
        assert!(p.remove("b"));
        p.deliver("b", BrainMsg::Delta("never".into()), &mut ());
        p.deliver("b", BrainMsg::Error { kind: "cancelled".into(), message: "cancelled".into() }, &mut ());
        p.deliver("unknown", BrainMsg::Done(Value::Null), &mut ());
        assert_eq!(*log.borrow(), vec!["a:delta:x", "a:delta:y", r#"a:done:{"text":"xy"}"#]);
        assert!(p.waiting.is_empty());
    }

    #[std::prelude::v1::test]
    fn a_stopped_brain_fails_every_waiting_callback_once() {
        let log: Log = Rc::default();
        let mut p: Pending<()> = Pending::new();
        for id in ["r1", "r2", "r3"] {
            p.insert(id.into(), recorder(id, &log));
        }
        p.deliver("r2", BrainMsg::Done(Value::Null), &mut ());
        p.fail_all("the brain stopped", &mut ());
        p.fail_all("again", &mut ());
        let mut got = log.borrow().clone();
        got.sort();
        assert_eq!(got, vec!["r1:error:other:the brain stopped", "r2:done:null", "r3:error:other:the brain stopped"]);
    }

    #[std::prelude::v1::test]
    fn events_are_read_from_their_lines() {
        let ev = |s: &str| parse_event(&serde_json::from_str(s).unwrap());
        assert_eq!(ev(r#"{"ev":"ready","provider":"claude","version":"0.1.0"}"#), Event::Ready);
        assert_eq!(ev(r#"{"ev":"delta","id":"r1","text":"hi"}"#), Event::Delta { id: "r1".into(), text: "hi".into() });
        assert_eq!(ev(r#"{"ev":"done","id":"r1","data":{"text":"hi"}}"#), Event::Done { id: "r1".into(), data: json!({"text": "hi"}) });
        assert_eq!(ev(r#"{"ev":"done","id":"r1"}"#), Event::Done { id: "r1".into(), data: Value::Null });
        assert_eq!(
            ev(r#"{"ev":"error","id":"r1","message":"boom"}"#),
            Event::Error { id: "r1".into(), kind: "other".into(), message: "boom".into() }
        );
        let signed_in = Some(Account { email: Some("a@b.c".into()), plan: Some("max".into()) });
        assert_eq!(
            ev(r#"{"ev":"status","id":"status-1","data":{"account":{"email":"a@b.c","subscriptionType":"max","tokenSource":"claude.ai"},"models":[]}}"#),
            Event::Status { account: signed_in.clone(), models: None },
            "an empty list is no list"
        );
        // No account (or not an object) is not "signed in as your Claude account".
        assert_eq!(ev(r#"{"ev":"status","id":"s","data":{"account":null}}"#), Event::Status { account: None, models: None });
        assert_eq!(ev(r#"{"ev":"status","id":"s","data":{}}"#), Event::Status { account: None, models: None });
        assert_eq!(ev(r#"{"ev":"status","id":"s","data":{"account":"x","models":null}}"#), Event::Status { account: None, models: None });
        assert_eq!(
            ev(r#"{"ev":"limit","status":"allowed_warning","resetsAt":1762430400,"utilization":0.91,"kind":"five_hour"}"#),
            Event::Limit(Limit { status: "allowed_warning".into(), resets_at: Some(1762430400), utilization: Some(0.91), kind: Some("five_hour".into()) })
        );
        // A fractional reset time still reads.
        assert_eq!(
            ev(r#"{"ev":"limit","status":"rejected","resetsAt":1762430400.5}"#),
            Event::Limit(Limit { status: "rejected".into(), resets_at: Some(1762430400), utilization: None, kind: None })
        );
        assert_eq!(ev(r#"{"ev":"limit"}"#), Event::Limit(Limit::default()));
        // A refusal holds until its reset, or a few minutes when the reset is unknown or past.
        let at = |resets_at| Limit { status: "rejected".into(), resets_at, ..Limit::default() };
        assert_eq!(hold_until(Some(&at(Some(2_000))), true, 1_000), Some(2_000));
        assert_eq!(hold_until(Some(&at(Some(500))), true, 1_000), Some(1_000 + HOLD_SECS));
        assert_eq!(hold_until(None, true, 1_000), Some(1_000 + HOLD_SECS));
        assert_eq!(hold_until(Some(&at(Some(2_000))), false, 1_000), None);
        // The brain cannot fake its own exit, and odd lines are ignored.
        assert_eq!(ev(r#"{"ev":"exit"}"#), Event::Unknown);
        assert_eq!(ev(r#"{"id":"r1"}"#), Event::Unknown);
        assert_eq!(ev("42"), Event::Unknown);
    }

    #[std::prelude::v1::test]
    fn the_model_list_is_read_from_the_sdk_shape() {
        let ev = |s: &str| parse_event(&serde_json::from_str(s).unwrap());
        let line = r#"{"ev":"status","id":"s","data":{"account":null,"models":[
            {"value":"default","displayName":"Default (recommended)","description":"Opus for most work"},
            {"value":"opus","resolvedModel":"claude-opus-x","displayName":"Opus","description":" Most capable ","supportsEffort":true,"supportedEffortLevels":["low","medium","high","xhigh","max","turbo"]},
            {"value":"haiku","displayName":"","supportsEffort":false,"supportedEffortLevels":["low"]},
            {"value":"opus","displayName":"Opus again"},
            {"displayName":"No value"},
            {"value":"  "},
            "not a model"
        ]}}"#;
        let Event::Status { models: Some(m), .. } = ev(line) else { panic!("no models") };
        assert_eq!(m.len(), 3);
        assert_eq!((m[0].value.as_str(), m[0].name.as_str(), m[0].efforts.clone()), ("default", "Default (recommended)", None));
        assert_eq!(m[1].description, "Most capable");
        assert_eq!(m[1].resolved.as_deref(), Some("claude-opus-x"));
        assert_eq!(m[1].efforts.clone().unwrap(), vec!["low", "medium", "high", "xhigh", "max"], "unknown levels are dropped");
        assert_eq!((m[2].name.as_str(), m[2].efforts.clone()), ("haiku", Some(vec![])), "no effort, and the value names it");
        assert_eq!(ev(r#"{"ev":"status","id":"s","data":{"models":[{"displayName":"x"}]}}"#), Event::Status { account: None, models: None });
    }

    /// What the real SDK's supportedModels() returned on 6 Nov 2026 (one status call).
    #[std::prelude::v1::test]
    fn the_real_sdk_list_reads() {
        let models = include_str!("../brain/test/fixtures/supported-models.json");
        let line = format!(r#"{{"ev":"status","id":"s","data":{{"account":null,"models":{models}}}}}"#);
        let Event::Status { models: Some(m), .. } = parse_event(&serde_json::from_str(&line).unwrap()) else { panic!("no models") };
        assert_eq!(m.len(), 13);
        assert_eq!((m[0].value.as_str(), m[0].name.as_str()), ("default", "Default (recommended)"));
        assert_eq!(m[0].efforts.as_deref().map(|e| e.len()), Some(5));
        assert!(m.iter().all(|c| !c.name.is_empty() && !c.description.is_empty() && c.resolved.is_some()));
        let haiku_45 = m.iter().find(|c| c.value == "claude-haiku-4-5-20251001").unwrap();
        assert_eq!(haiku_45.efforts, None, "no levels reported");
        // A setting saved before the list existed shows as its alias, with its levels.
        let i = crate::store::chosen_model(&m, "claude-opus-5-5").unwrap();
        assert_eq!(m[i].value, "opus");
        assert_eq!(super::effort_for(&m, "claude-opus-5-5", "xhigh"), "xhigh");
        assert_eq!(super::effort_for(&m, "claude-sonnet-4-6", "xhigh"), "high");
        assert_eq!(m.iter().filter(|c| !c.is_pinned()).count(), 5, "five names that move with releases");
    }

    #[std::prelude::v1::test]
    fn effort_follows_what_the_model_takes() {
        use crate::store::ModelChoice;
        let m = |value: &str, efforts: Option<&[&str]>| ModelChoice {
            value: value.into(),
            efforts: efforts.map(|e| e.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };
        let list = vec![
            m("default", Some(&["low", "medium", "high"])),
            m("big", Some(&["low", "medium", "high", "xhigh", "max"])),
            m("small", Some(&[])),
            m("quiet", None),
            m("odd", Some(&["low", "max"])),
        ];
        assert_eq!(super::effort_for(&list, "big", "xhigh"), "xhigh");
        assert_eq!(super::effort_for(&list, "", "max"), "high", "the default row stands for an empty setting");
        assert_eq!(super::effort_for(&list, "default", "xhigh"), "high");
        assert_eq!(super::effort_for(&list, "small", "high"), "", "a model without levels gets none");
        assert_eq!(super::effort_for(&list, "quiet", "high"), "high", "no levels reported: send what was asked");
        assert_eq!(super::effort_for(&list, "unknown", "medium"), "medium");
        assert_eq!(super::effort_for(&list, "odd", "xhigh"), "max");
        assert_eq!(super::effort_for(&list, "odd", "medium"), "low");
        assert_eq!(super::effort_for(&list, "odd", "high"), "low", "a tie goes to the lower level");
        assert_eq!(super::effort_for(&[], "big", "low"), "low");
    }

    #[std::prelude::v1::test]
    fn garbage_on_stdout_is_skipped_without_ending_the_stream() {
        let big = "x".repeat(3 << 20);
        let mut input: Vec<u8> = vec![];
        input.extend_from_slice(b"Debugger listening on ws://...\n\n   \n");
        input.extend_from_slice(b"{\"ev\":\"ready\"}\r\n");
        input.extend_from_slice(b"\xff\xfe{\"ev\":\"delta\",\"id\":\"r1\",\"text\":\"bad bytes\"}\n");
        // serde_json refuses a lone surrogate escape; that line alone is lost.
        input.extend_from_slice(b"{\"ev\":\"delta\",\"id\":\"r1\",\"text\":\"\\ud83d\"}\n");
        input.extend_from_slice(format!("{{\"ev\":\"delta\",\"id\":\"r1\",\"text\":\"{big}\"}}\n").as_bytes());
        input.extend_from_slice("{\"ev\":\"delta\",\"id\":\"r1\",\"text\":\"漢字 🚀\"}\n".as_bytes());
        input.extend_from_slice(b"{\"ev\":\"done\",\"id\":\"r1\",\"data\":{}}"); // no final newline
        let mut got = vec![];
        pump(&input[..], |v| {
            got.push(v);
            true
        });
        let kinds: Vec<_> = got.iter().map(|v| v["ev"].as_str().unwrap().to_string()).collect();
        assert_eq!(kinds, vec!["ready", "delta", "delta", "done"]);
        assert_eq!(got[1]["text"].as_str().unwrap().len(), big.len());
        assert_eq!(got[2]["text"], "漢字 🚀");
    }

    #[std::prelude::v1::test]
    fn the_sink_can_stop_the_pump() {
        let input = b"{\"ev\":\"ready\"}\n{\"ev\":\"ready\"}\n{\"ev\":\"ready\"}\n";
        let mut n = 0;
        pump(&input[..], |_| {
            n += 1;
            n < 2
        });
        assert_eq!(n, 2);
    }

    #[std::prelude::v1::test]
    fn launch_problems_say_what_is_wrong() {
        let dir = scratch("launch");
        let err = |l: &Launch| format!("{:#}", l.command().err().expect("expected an error"));
        let base = Launch { dir: dir.clone(), entry: None, bun: Some("/bin/echo".into()), log: dir.join("brain.log") };

        let missing = Launch { entry: Some("test/nope.ts".into()), ..base.clone() };
        assert!(err(&missing).contains("INKLING_BRAIN_ENTRY"), "{}", err(&missing));
        assert!(err(&missing).contains("nope.ts"));

        assert!(err(&base).contains("no src/main.ts"), "{}", err(&base));
        let gone = Launch { dir: dir.join("does-not-exist"), ..base.clone() };
        assert!(err(&gone).contains("does-not-exist"));

        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.ts"), "").unwrap();
        let no_bun = Launch { bun: None, ..base.clone() };
        assert!(err(&no_bun).starts_with("Bun is not installed"), "{}", err(&no_bun));
        let cmd = base.command().unwrap();
        assert_eq!(cmd.get_program(), "/bin/echo");
        assert_eq!(cmd.get_args().collect::<Vec<_>>(), vec!["run", "src/main.ts"]);
        assert_eq!(cmd.get_current_dir(), Some(dir.as_path()));

        // A compiled brain needs no Bun.
        std::fs::create_dir_all(dir.join("dist")).unwrap();
        std::fs::write(dir.join("dist/inkling-brain"), "").unwrap();
        let cmd = Launch { bun: None, ..base.clone() }.command().unwrap();
        assert_eq!(cmd.get_program(), dir.join("dist/inkling-brain").as_os_str());
    }

    #[std::prelude::v1::test]
    fn the_crash_line_is_found_in_bun_output() {
        let missing_pkg = "[brain] task read r1 failed (other): x\nerror: Cannot find package '@anthropic-ai/claude-agent-sdk' from '/a/src/claude.ts'\n\nBun v1.4.2 (macOS arm64)\n";
        assert_eq!(crash_reason(missing_pkg).as_deref(), Some("error: Cannot find package '@anthropic-ai/claude-agent-sdk' from '/a/src/claude.ts'"));
        let thrown = "152 | async function handle(req: Request) {\n153 |   switch (req.op) {\n              ^\nTypeError: null is not an object (evaluating 'req.op')\n      at handle (/a/src/main.ts:153:11)\n\nBun v1.4.2 (macOS arm64)\n";
        assert_eq!(crash_reason(thrown).as_deref(), Some("TypeError: null is not an object (evaluating 'req.op')"));
        assert_eq!(crash_reason("[brain] task read r1 failed (auth): Not logged in\n"), None);
        assert_eq!(crash_reason("Bun v1.4.2 (macOS arm64)\n"), None);
        assert_eq!(crash_reason(""), None);
    }

    /// A fake brain made of a shell script, installed as the compiled brain.
    fn script_brain(name: &str, body: &str) -> Launch {
        let dir = scratch(name);
        std::fs::create_dir_all(dir.join("dist")).unwrap();
        let path = dir.join("dist/inkling-brain");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::process::Command::new("chmod").arg("+x").arg(&path).status().unwrap();
        Launch { dir: dir.clone(), entry: None, bun: None, log: dir.join("brain.log") }
    }

    /// Spawn, and pump its stdout into a channel (`None` when it closes).
    fn start(launch: &Launch, cmd: std::process::Command) -> (Proc, mpsc::Receiver<Option<Value>>) {
        let (proc, stdout) = Proc::spawn(cmd, &launch.log).unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            pump(stdout, |v| tx.send(Some(v)).is_ok());
            let _ = tx.send(None);
        });
        (proc, rx)
    }

    fn next(rx: &mpsc::Receiver<Option<Value>>) -> Option<Value> {
        rx.recv_timeout(Duration::from_secs(20)).expect("the brain went quiet")
    }

    #[std::prelude::v1::test]
    fn a_brain_that_dies_mid_request_fails_the_waiters_and_says_why() {
        let launch = script_brain(
            "dies",
            r#"echo '{"ev":"ready","provider":"fake","version":"0"}'
read line
echo '{"ev":"delta","id":"r1","text":"par"}'
echo "error: the fake brain fell over" >&2
exit 3"#,
        );
        let (mut proc, rx) = start(&launch, launch.command().unwrap());
        assert_eq!(parse_event(&next(&rx).unwrap()), Event::Ready);
        let log: Log = Rc::default();
        let mut pending: Pending<()> = Pending::new();
        pending.insert("r1".into(), recorder("r1", &log));
        pending.insert("r2".into(), recorder("r2", &log));
        proc.send(&json!({"op": "run", "id": "r1"})).unwrap();
        loop {
            match next(&rx) {
                Some(v) => {
                    if let Event::Delta { id, text } = parse_event(&v) {
                        pending.deliver(&id, BrainMsg::Delta(text), &mut ());
                    }
                }
                None => break,
            }
        }
        let why = proc.reap();
        assert!(why.starts_with("the brain stopped (exit status: 3)"), "{why}");
        assert!(why.ends_with("error: the fake brain fell over"), "{why}");
        pending.fail_all(&why, &mut ());
        let mut got = log.borrow().clone();
        got.sort();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], "r1:delta:par");
        assert!(got[1].starts_with("r1:error:other:the brain stopped"));
        assert!(got[2].starts_with("r2:error:other:the brain stopped"));

        // Writing to a dead brain is an error, not a panic or a hang.
        let quits = script_brain("quits", "exit 0");
        let (mut dead, rx) = start(&quits, quits.command().unwrap());
        while next(&rx).is_some() {}
        let _ = dead.child.wait();
        let mut failed = false;
        for _ in 0..50 {
            if dead.send(&json!({"op": "run", "id": "x", "pad": "x".repeat(65536)})).is_err() {
                failed = true;
                break;
            }
        }
        assert!(failed, "writes to a dead brain kept succeeding");
    }

    /// What the brain's draw task returns has to get through the app's XML parser.
    #[std::prelude::v1::test]
    fn drawings_from_the_brain_parse_in_the_renderer() {
        // Real drawings from the model (brain/test/live.ts), as cleanSvg returned them.
        for svg in [
            include_str!("../brain/test/fixtures/live-rate-limiter.svg"),
            include_str!("../brain/test/fixtures/live-injected-note.svg"),
        ] {
            let d = crate::drawing::parse(svg).expect("a live drawing did not parse");
            assert!((2..=6).contains(&d.steps.len()), "{} steps", d.steps.len());
            for s in &d.steps {
                assert!(!s.caption.trim().is_empty() && !s.shapes.is_empty());
            }
        }
        // Model output with HTML entities and bare `&` / `<`: the old cut (first <svg to
        // last </svg>) does not parse; cleanSvg's output does, with the captions intact.
        let raw = include_str!("../brain/test/fixtures/messy-model-output.txt");
        let old_cut = &raw[raw.find("<svg").unwrap()..raw.rfind("</svg>").unwrap() + 6];
        assert!(crate::drawing::parse(old_cut).is_none());
        let cleaned = crate::drawing::parse(include_str!("../brain/test/fixtures/messy-cleaned.svg")).expect("cleaned drawing");
        let captions: Vec<_> = cleaned.steps.iter().map(|s| s.caption.as_str()).collect();
        assert_eq!(captions, vec!["R&D window → reset", "100 + 100 < 2 s"]);
    }

    #[std::prelude::v1::test]
    fn write_line_writes_one_json_line() {
        let mut out = vec![];
        write_line(&mut out, &json!({"op": "cancel", "id": "r1"})).unwrap();
        assert_eq!(out, b"{\"op\":\"cancel\",\"id\":\"r1\"}\n");
    }

    fn bun() -> Option<PathBuf> {
        super::find_bun()
    }

    /// The real brain with a fake SDK (brain/test/fake-brain.ts), as INKLING_BRAIN_ENTRY runs it.
    #[std::prelude::v1::test]
    fn the_real_brain_over_the_wire_with_a_fake_model() {
        let Some(bun) = bun() else {
            eprintln!("skipping: no bun");
            return;
        };
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("brain");
        let data = scratch("fake-brain-data");
        let launch = Launch { dir: dir.clone(), entry: Some("test/fake-brain.ts".into()), bun: Some(bun), log: data.join("brain.log") };
        let spawn = |launch: &Launch| {
            let mut cmd = launch.command().unwrap();
            cmd.env("INKLING_DATA_DIR", &data).env_remove("ANTHROPIC_API_KEY");
            start(launch, cmd)
        };
        let (mut proc, rx) = spawn(&launch);
        let fake = |d: Value| format!("[[fake {d}]]");
        let ctx = |note: String| json!({"noteTitle": "t", "noteText": note, "brief": null, "aboutMe": null, "threads": [],
            "now": "today", "mind": {"name": "Inkling", "description": "", "howItThinks": "", "voice": "plain", "frequency": "often"}});
        let run = |id: &str, note: String| {
            json!({"op": "run", "id": id, "settings": {"model": "m", "effort": "low"},
                "input": {"task": "ask", "ctx": ctx(note), "target": "words", "question": "why?"}})
        };
        let log: Log = Rc::default();
        let mut pending: Pending<()> = Pending::new();
        pending.insert("r1".into(), recorder("r1", &log));
        pending.insert("r2".into(), recorder("r2", &log));
        proc.send(&json!({"op": "status", "id": "status-1"})).unwrap();
        proc.send(&run("r1", fake(json!({"deltas": ["Hel", "lo"], "delayMs": 10})))).unwrap();
        proc.send(&run("r2", fake(json!({"deltas": ["a", "b", "c", "d", "e", "f"], "delayMs": 150})))).unwrap();
        let mut status = None;
        let mut limit_seen = false;
        let mut r2_errors = 0;
        loop {
            let v = next(&rx).expect("the brain stopped early");
            match parse_event(&v) {
                Event::Ready => {}
                Event::Status { account, models } => status = Some((account, models)),
                Event::Limit(_) => limit_seen = true,
                Event::Delta { id, text } => {
                    // Cancel r2 as soon as it starts talking.
                    if id == "r2" && pending.remove("r2") {
                        proc.send(&json!({"op": "cancel", "id": "r2"})).unwrap();
                    }
                    pending.deliver(&id, BrainMsg::Delta(text), &mut ());
                }
                Event::Done { id, data } => pending.deliver(&id, BrainMsg::Done(data), &mut ()),
                Event::Error { id, kind, message } => {
                    if id == "r2" {
                        assert_eq!(kind, "cancelled");
                        r2_errors += 1;
                    }
                    pending.deliver(&id, BrainMsg::Error { kind, message }, &mut ())
                }
                Event::Unknown => panic!("unexpected line {v}"),
            }
            if r2_errors == 1 && status.is_some() && log.borrow().iter().any(|l| l.starts_with("r1:done")) {
                break;
            }
        }
        assert!(!limit_seen);
        let (account, models) = status.unwrap();
        assert_eq!(account, Some(Account { email: Some("writer@example.com".into()), plan: Some("max".into()) }));
        let models = models.expect("the fake SDK lists a model");
        assert_eq!((models[0].value.as_str(), models[0].name.as_str()), ("claude-opus-5-5", "Opus 5.5"));
        assert_eq!(*log.borrow(), vec!["r1:delta:Hel", "r1:delta:lo", r#"r1:done:{"text":"Hello"}"#]);

        // Closing its stdin stops it; a new one starts cleanly afterwards.
        proc.stop();
        while next(&rx).is_some() {}
        let (mut again, rx) = spawn(&launch);
        assert_eq!(parse_event(&next(&rx).unwrap()), Event::Ready);
        again.send(&run("r3", fake(json!({"result": "back"})))).unwrap();
        loop {
            if let Event::Done { id, data } = parse_event(&next(&rx).unwrap()) {
                assert_eq!(id, "r3");
                assert_eq!(data, json!({"text": "back"}));
                break;
            }
        }
        again.stop();
    }
}

#[cfg(test)]
#[path = "brain/robustness_tests.rs"]
mod robustness_tests;
