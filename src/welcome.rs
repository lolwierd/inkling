//! The first note a new notebook opens with. It shows a margin thread and a
//! drawing straight away, so the idea lands before any model call.

use chrono::Utc;

use crate::store::{
    Block, BlockKind, Callout, Check, Given, Lang, Message, Mode, Note, Passage, Role, Side, SumLine, Thread, ThoughtKind,
    ThreadStatus, new_id,
};

/// The old welcome note's sketch; only tests open that note now.
#[cfg(test)]
const MOON_SKETCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 236">
<g data-step="1" data-caption="Sunlight always arrives from one side.">
<text x="46" y="40" class="voice" font-size="17" fill="ink" text-anchor="middle">Sun</text>
<path d="M22 78 Q50 76 84 78 M22 118 Q50 116 84 118 M22 158 Q50 156 84 158" stroke="graphite" stroke-width="1.1" fill="none"/>
<path d="M78 74 L84 78 L78 82 M78 114 L84 118 L78 122 M78 154 L84 158 L78 162" stroke="graphite" stroke-width="1.1" fill="none"/>
</g>
<g data-step="2" data-caption="The Moon turns once per orbit, so one face always points at Earth.">
<circle cx="262" cy="118" r="28" stroke="ink" stroke-width="1.4" fill="none"/>
<text x="262" y="170" class="voice" font-size="15" fill="ink" text-anchor="middle">Earth</text>
<circle cx="164" cy="118" r="15" stroke="ink" stroke-width="1.3" fill="none"/>
<text x="164" y="152" class="sans" font-size="11" fill="graphite" text-anchor="middle">Moon</text>
<path d="M184 113 Q207 109 230 113" stroke="faint" stroke-width="1" fill="none"/>
<text x="207" y="101" class="sans" font-size="10" fill="faint" text-anchor="middle">near side</text>
</g>
<g data-step="3" data-caption="At new Moon, the far side is in full daylight.">
<path d="M164 103 A15 15 0 0 1 164 133 Z" stroke="none" fill="graphite"/>
<text x="104" y="210" class="voice" font-size="15" fill="ink" text-anchor="middle">far side: day</text>
<path d="M112 194 Q128 162 150 128" stroke="ink" stroke-width="1.1" fill="none"/>
<path d="M144 131 L150 128 L151 134" stroke="ink" stroke-width="1.1" fill="none"/>
<text x="220" y="210" class="voice" font-size="15" fill="ink" text-anchor="middle">near side: night</text>
<path d="M214 194 Q196 164 176 130" stroke="ink" stroke-width="1.1" fill="none"/>
</g>
</svg>"#;

/// The guide's sketch, on its boot-order correction: three stages of a cold start, laid
/// end to end on a time line, then the brace that sums them.
const BOOT_SKETCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 176">
<g data-step="1" data-caption="Firmware runs first, then hands over to the kernel.">
<path d="M20 96 L300 96" stroke="graphite" stroke-width="1.1" fill="none"/>
<path d="M294 92 L300 96 L294 100" stroke="graphite" stroke-width="1.1" fill="none"/>
<text x="300" y="113" class="sans" font-size="10" fill="faint" text-anchor="end">time</text>
<rect x="20" y="44" width="78" height="28" rx="4" stroke="ink" stroke-width="1.4" fill="none"/>
<text x="59" y="32" class="voice" font-size="15" fill="ink" text-anchor="middle">firmware</text>
</g>
<g data-step="2" data-caption="The kernel starts systemd, and only then do the services begin.">
<rect x="102" y="44" width="58" height="28" rx="4" stroke="ink" stroke-width="1.4" fill="none"/>
<text x="131" y="32" class="voice" font-size="15" fill="ink" text-anchor="middle">kernel</text>
<rect x="164" y="44" width="132" height="28" rx="4" stroke="ink" stroke-width="1.4" fill="none"/>
<text x="230" y="32" class="voice" font-size="15" fill="ink" text-anchor="middle">services</text>
<path d="M100 76 L100 92 M162 76 L162 92" stroke="faint" stroke-width="1" fill="none"/>
</g>
<g data-step="3" data-caption="So a boot is the three added up, not the slowest of them.">
<path d="M20 118 Q20 128 30 128 L148 128 Q158 128 158 138 Q158 128 168 128 L286 128 Q296 128 296 118" stroke="ink" stroke-width="1.1" fill="none"/>
<text x="158" y="162" class="voice" font-size="15" fill="ink" text-anchor="middle">boot = all three, in turn</text>
</g>
</svg>"#;

/// The welcome note before the guide: a few lines and the Moon correction. Only tests
/// (export, drawing, store) still open it.
#[cfg(test)]
pub fn welcome_note(mode: Mode) -> Note {
    let mut note = Note::new("Welcome to Inkling", mode, None);
    let quote = "The far side of the Moon is dark because sunlight never reaches it.";
    note.blocks = vec![
        Block { kind: BlockKind::Title, text: "Welcome to Inkling".into() },
        Block {
            kind: BlockKind::Paragraph,
            text: "Write anything. Inkling reads along and writes in the margin when something is worth a second look.".into(),
        },
        Block {
            kind: BlockKind::Paragraph,
            text: "Rather it waited? Switch a note to Waits to be asked, in the row above its title. Then select some words, call it by name, or open the chat with ⌘J.".into(),
        },
        Block { kind: BlockKind::Paragraph, text: quote.into() },
        Block { kind: BlockKind::Paragraph, text: String::new() },
    ];
    let joined = note.joined_text();
    let start = joined.find(quote).unwrap_or(0);
    note.threads.push(Thread {
        id: new_id(),
        start,
        end: start + quote.len(),
        quote: quote.into(),
        kind: ThoughtKind::Correction,
        messages: vec![Message::new(
            Role::Mind,
            "It gets as much sunlight as the side we see. It is only hidden from us: the Moon turns once per orbit, so one face always points at Earth. At new Moon, which side is lit?",
        )],
        drawing: Some(MOON_SKETCH.into()),
        status: ThreadStatus::Open,
        side: Side::Left,
        mind_name: "Inkling".into(),
        tag_offset: (0., 0.),
        created_at: Utc::now(),
        reason: None,
        ended_at: None,
        first_quote: None,
        ..Default::default()
    });
    for b in note.blocks.iter().skip(1) {
        note.read_hashes.push(crate::store::hash_text(&b.text));
    }
    note
}

/// The first note of a new notebook, and the Welcome Guide in the Help menu: a short
/// tour that is itself a working note. Its threads are real (point at them, answer
/// them, fix the words and watch one tie off): a correction with its working, one that
/// rests on the note's own words, a question, a thread of your own in ink, two minds on
/// one line, and a point already let stand, whose reason is on the back of the note. It
/// ends by asking for one sentence of your own, which Inkling reads with a question first
/// (`first_run`).
pub fn first_run_note(mode: Mode) -> Note {
    use BlockKind::{Bullet, Check as Item, Code, H2, Paragraph as P, Quote, Title};
    let rust = Code(Lang::new("rust"));
    let mut note = Note::new("Start here", mode, None);
    // one running example, a plan to make virtual machines start faster
    let image = "Reading a 20 GB disk image at 400 MB/s takes 5 seconds.";
    let faster = "Restoring a snapshot is always faster than booting.";
    let staging = "We try it on the staging hosts first, from Monday.";
    let enough = "A warm pool of ten VMs per host is enough for Monday mornings.";
    let order = "Firmware, the kernel and the services start side by side, so a boot takes as long as the slowest of them.";
    let timed = "Time one cold start with systemd-analyze";
    let added = "firmware + kernel + services // one after another";
    let blocks: Vec<(BlockKind, &str)> = vec![
        (Title, "Start here"),
        (P, "Inkling is a notebook that asks. You write, and it reads along. When a line doesn\u{2019}t add up, it pulls a thread from those words into the margin: a question, or a correction. It never writes your notes for you. The thinking stays yours."),
        (P, "Everything on this page works. Point at a tag in the margin to read it, click it to answer, and drag it anywhere you like."),
        (H2, "Threads"),
        (P, image),
        (P, faster),
        (P, "A \u{d7} on a tag is a correction and a ? is a question. Open questions are numbered in reading order, with a knot beside their paragraph; the count above the title says how many are loose, and \u{2325}\u{2318}\u{2193} jumps to the next one. Fix the words a thread is about and Inkling reads them again; if the problem is gone, the thread ties itself off and its knot joins the border at the top of the page."),
        (P, "A correction shows its working, so you can check it yourself. Under the one about the disk image is a sum made only from your numbers, each linked to the words it came from; the one about boot order further down points to your own words elsewhere in the note. When Inkling cannot show why, it asks a question instead. If the working is wrong, press Disagree, and if Inkling agrees it takes the correction back."),
        (P, "Open a tag to write back, and the mind answers in the same thread. Resolve ties it off. Let stand leaves a mind\u{2019}s point as it is: what you typed in its box goes with it as your reason, and no mind raises the point again. Your own threads, asks and sketches have Dismiss instead, which snips them. Undo, or \u{2325}\u{2318}Z, brings either back. Copy takes the whole thread with you, and Make a note turns it into a note of its own."),
        (P, "You can also answer in the note itself. Write the answer as a paragraph anywhere on the page, and once the thread\u{2019}s mind agrees, the thread ties off and a knot sews in after your words. When a reply in a tag settles a thread, the tag offers Write it in, to put your reply under its paragraph."),
        (P, "A thread can join two notes. When what you write disagrees with words in another note, or builds on them, the thread quotes both, and Go there (\u{2325}\u{2318}\u{2192}) opens the other note at its words."),
        (P, "Turn over, above the title (\u{21e7}\u{2318}B), shows the back of the note: every thread it has had, how each one ended, and your reasons, with Reopen to bring one back. One thread on this page is already let stand, and the back says why."),
        (H2, "Reading along"),
        (P, "Inkling reads each paragraph when you pause, and a small stitch in the gutter marks what it has read. Most of the time it says nothing. A question that comes while you are mid-sentence waits, folded, until you stop."),
        (P, "Tell Inkling, above the title, says what a note is for, and every read takes it into account. Beside it, Reads along can become Waits to be asked: then Inkling stays quiet until you call it, select words, or open the chat."),
        (P, "The last item in that menu, Read it through (\u{21e7}\u{2318}R), reads every paragraph not read yet, top to bottom, once. Threads arrive as it goes, and \u{21e7}\u{2318}R again stops it."),
        (H2, "Asking"),
        (P, "Select some words and a small menu appears. Find a loose thread (\u{2318}\u{21b5}) picks the one thing in them most worth a question. Ask Inkling takes your own question. Sketch it answers with a drawing, stitched out step by step; click a sketch to see it large. The same menu turns a line into a heading, a list, a quote, a callout or code."),
        (P, "Ask yourself (\u{21e7}\u{2318}\u{21b5}) hangs a question of your own on the words, sewn in your ink instead of a mind\u{2019}s colour. No mind answers it unless you call one by name, and only you tie it off. The next line has one."),
        (P, staging),
        (P, enough),
        (P, "Ask the room to hear from up to three minds at once. Each answers on its own thread, in its own colour. Or start a sentence with a mind\u{2019}s name and a comma, and that mind answers you there."),
        (P, order),
        (P, "If you ask, Inkling can look a fact up on the web when the note cannot settle it: in Find a loose thread, Ask Inkling, a mind called by name, a reply or the chat. A page it cites shows only when the words it quotes are on that page. Reading along never searches, and Settings, under Model, turns it off."),
        (P, "\u{2318}J opens a chat about the whole note. It has read every thread as well, so it can tell you what you have settled and what is still open."),
        (H2, "Checklists, callouts and code"),
        (P, "Start a line with [ ] and a space for a checklist item; click its box, or press \u{2318}L, to check it off. Start one with > [!note], > [!warning] or > [!tip] and a space for a callout. Three backticks and a language, then Return, start a block of code; Return twice at its end leaves it, and \u{2318}E sets a few words as code inside a line. Inkling reads all of them like any paragraph, and can ask about a single line of code."),
        (Item(true), timed),
        (Item(false), "Restore one VM from a snapshot, nothing else"),
        (BlockKind::Callout(Callout::Tip), "Time cold starts at p99, not the average: the slow ones are the ones people notice."),
        (rust, "fn boot_ms(firmware: u32, kernel: u32, services: u32) -> u32 {"),
        (rust, "    firmware + kernel + services // one after another"),
        (rust, "}"),
        (Quote, "The fastest boot is the one you skip."),
        (H2, "Minds"),
        (P, "The name above the title is the note\u{2019}s mind. Inkling comes with Staff engineer, Editor, Skeptic, Socratic tutor and Investor. In Settings, under Minds, you can make your own: say what it is for and how it thinks, give it a voice (plain, warm, blunt or Socratic), and choose how often it speaks. Each mind keeps its own thread colour, so you can see who asked what."),
        (P, "Settings, under About you, holds what Inkling knows about you. Turn on Inkling learns about you, and it keeps what matters from your replies."),
        (H2, "Your notebook"),
        (P, "\u{2318}\\ opens your notes and folders; a dot marks notes with open threads. \u{2318}K finds a note or starts one. \u{21e7}\u{2318}C copies a note, and \u{21e7}\u{2318}E exports it as Markdown, HTML or Word, with its threads or without. Help > Welcome Guide brings this page back."),
        (P, "\u{21e7}\u{2318}K shows your loose ends: every open thread in the notebook, oldest first. Type to narrow the list, and Return opens the note at those words."),
        (P, "Coming from Obsidian? File > Import from Obsidian brings a vault in with its folders. Inkling leaves the imported words alone, so it reads only what you write afterwards, until you have a note read through."),
        (H2, "Make it yours"),
        (P, "In Settings (\u{2318},) choose Light, Dark or the system\u{2019}s look, the text size, the lettering, and your thread colour: madder, indigo, sage, rose, plum, walnut or ochre. Questions can appear in both margins, in the right margin only, or as small marks in the text. Reading pace can be regular or slow. Ink from the pointer lets a drop of ink fall now and then as you move it."),
        (P, "Hold \u{2325} on its own to speak instead of typing. Under Model, pick which Claude model Inkling uses, and how much effort it spends reading along and answering. It runs on the Claude login already on this Mac."),
        (P, "Under Shortcuts you can change any key: click it and press the keys you want. Everything works from the keyboard: Tab and \u{21e7}Tab move between controls, Return or Space presses one, and \u{2303}Tab leaves the note."),
        (H2, "Quiet when you want it"),
        (P, "The window\u{2019}s controls fade while you write and come back when you move the pointer. \u{21e7}\u{2318}. hides every thread and leaves just your words."),
        (H2, "Keys"),
        (Bullet, "\u{2318}\u{21b5} finds a loose thread in what you selected."),
        (Bullet, "\u{21e7}\u{2318}\u{21b5} hangs a question of your own on what you selected."),
        (Bullet, "\u{2325}\u{2318}\u{2193} and \u{2325}\u{2318}\u{2191} move between open threads."),
        (Bullet, "\u{2325}\u{2318}\u{2192} goes to the other end of a thread between notes."),
        (Bullet, "\u{21e7}\u{2318}\u{232b} lets the open thread stand, or snips one of your own, and \u{2325}\u{2318}Z brings it back."),
        (Bullet, "\u{21e7}\u{2318}R reads the note through, once."),
        (Bullet, "\u{21e7}\u{2318}B turns the note over, and back."),
        (Bullet, "\u{21e7}\u{2318}K shows every loose end in the notebook."),
        (Bullet, "\u{21e7}\u{2318}. hides every thread and leaves just your words."),
        (Bullet, "\u{2318}J opens a chat about the whole note."),
        (Bullet, "\u{2318}L checks or unchecks a checklist item, and \u{2318}E sets words as code."),
        (Bullet, "\u{2318}K finds a note, or starts one."),
        (Bullet, "\u{2318}\\ shows your notes and folders."),
        (Bullet, "\u{21e7}\u{2318}C copies the note, and \u{21e7}\u{2318}E exports it."),
        (Bullet, "Hold \u{2325} to speak instead of typing."),
        (Bullet, "\u{2318}, opens Settings."),
        (H2, "Your turn"),
        (P, "Write one sentence below about something you\u{2019}re working on, or something you believe. Inkling will read it and ask you about it."),
        (P, ""),
    ];
    note.blocks = blocks.iter().map(|(kind, text)| Block { kind: *kind, text: (*text).into() }).collect();
    // Inkling leaves the guide's own words alone; only what you write is read
    note.left_alone = note.blocks.iter().skip(1).filter(|b| !b.text.is_empty()).map(|b| crate::store::hash_text(&b.text)).collect();

    let joined = note.joined_text();
    let thread = |quote: &str, kind: ThoughtKind, side: Side, mind: &str, messages: Vec<Message>, drawing: Option<&str>| {
        let start = joined.find(quote).unwrap_or(0);
        Thread {
            id: new_id(),
            start,
            end: start + quote.len(),
            quote: quote.into(),
            kind,
            messages,
            drawing: drawing.map(Into::into),
            status: ThreadStatus::Open,
            side,
            mind_name: mind.into(),
            tag_offset: (0., 0.),
            created_at: Utc::now(),
            reason: None,
            ended_at: None,
            first_quote: None,
            ..Default::default()
        }
    };
    let mind = |text: &str| Message::new(Role::Mind, text);
    let you = |text: &str| Message::new(Role::You, text);
    let checked = |text: &str, check: Check| Message { check: Some(check), ..mind(text) };
    // Show its working, as the brain makes sure of it (`brain/src/working.ts`): every
    // number in the sum is the guide's own or a unit, and every result is right
    let sum = |sum: &str, is: &str| SumLine { sum: sum.into(), is: is.into(), rounded: false };
    let given = |value: &str, words: &str| Given { value: value.into(), words: words.into() };
    let working = Check {
        sum: vec![sum("20 \u{d7} 1000 \u{f7} 400", "50"), sum("20 \u{f7} 5", "4")],
        givens: vec![given("20", "20 GB"), given("400", "400 MB/s"), given("5", "5 seconds")],
        ..Default::default()
    };
    // the writer's own code says the stages add up
    let own_words = Check { words: vec![Passage { words: added.into(), note: None }], ..Default::default() };
    // a question of your own, in your ink, and a point you let stand, with your reason
    let yours = |quote: &str, question: &str| Thread { own: true, ..thread(quote, ThoughtKind::Question, Side::Right, "", vec![you(question)], None) };
    let stood = |quote: &str, point: &str, reason: &str| {
        let mut t = thread(quote, ThoughtKind::Question, Side::Right, "Inkling", vec![mind(point)], None);
        t.set_status(ThreadStatus::Dismissed);
        t.set_reason(reason);
        t
    };
    // made in reading order, so the back of the note lists them in that order too
    note.threads = vec![
        thread(
            "takes 5 seconds",
            ThoughtKind::Correction,
            Side::Right,
            "Inkling",
            vec![checked("At 400 MB/s, 20 GB takes 50 seconds. Reading it in 5 needs 4 GB/s. Change the number in the sentence and this thread ties itself off.", working)],
            None,
        ),
        thread(
            faster,
            ThoughtKind::Question,
            Side::Left,
            "Inkling",
            vec![mind("Always? A restore skips the boot, but a guest that has to fault in most of its memory can end up slower than a lean boot. Which have you timed?")],
            None,
        ),
        yours("the staging hosts", "Is staging busy enough to show the slow starts?"),
        thread(
            enough,
            ThoughtKind::Ask,
            Side::Right,
            "Skeptic",
            vec![
                you("Is this enough?"),
                mind("Enough for which minute? A pool sized for the average empties in the first seconds of a burst. How many starts does one host see at nine on a Monday, and how fast does the pool refill?"),
            ],
            None,
        ),
        thread(
            enough,
            ThoughtKind::Ask,
            Side::Right,
            "Socratic tutor",
            vec![
                you("Is this enough?"),
                mind("If twenty requests reach one host in the same second, what happens to the eleventh? What would you measure to find out?"),
            ],
            None,
        ),
        thread(
            order,
            ThoughtKind::Correction,
            Side::Left,
            "Inkling",
            vec![checked("They run one after another: firmware hands over to the kernel, the kernel starts systemd, and systemd brings up the services. The times add up. Which of the three takes longest on yours?", own_words)],
            Some(BOOT_SKETCH),
        ),
        stood(timed, "One start is one sample. How many runs before you trust the number?", "A first look at where the time goes. The p99 runs come after."),
    ];
    note.first_run = true;
    note
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn the_guides_threads_sit_on_their_words() {
        let note = first_run_note(Mode::ThinksWithYou);
        let joined = note.joined_text();
        assert_eq!(note.threads.len(), 7);
        for t in &note.threads {
            assert_eq!(&joined[t.start..t.end], t.quote, "{}", t.quote);
        }
        // made in reading order, so the back of the note lists them in that order
        assert!(note.threads.windows(2).all(|w| w[0].start <= w[1].start && w[0].created_at <= w[1].created_at));
        // a tag in each margin, so the tour shows both
        assert!(note.threads.iter().any(|t| t.side == Side::Left) && note.threads.iter().any(|t| t.side == Side::Right));
        // the guide's own words are never read, so none of its threads gets a stray reply
        assert_eq!(note.left_alone.len(), note.blocks.iter().skip(1).filter(|b| !b.text.is_empty()).count());
    }

    #[std::prelude::v1::test]
    fn the_guides_sketch_parses() {
        let note = first_run_note(Mode::ThinksWithYou);
        let drawn: Vec<_> = note.threads.iter().filter_map(|t| t.drawing.as_deref()).collect();
        assert_eq!(drawn.len(), 1, "one thread answers with a sketch");
        let d = crate::drawing::parse(drawn[0]).expect("the guide's sketch parses");
        assert_eq!((d.min_x, d.min_y, d.width, d.height), (0., 0., 320., 176.));
        assert_eq!(d.steps.len(), 3);
        assert!(d.steps.iter().all(|s| !s.caption.is_empty() && !s.shapes.is_empty()));
    }

    /// A left-to-right sum of numbers, × and ÷, as the guide's working writes it.
    fn work_out(sum: &str) -> f64 {
        let mut parts = sum.split_whitespace();
        let mut v: f64 = parts.next().unwrap().parse().unwrap();
        while let (Some(op), Some(n)) = (parts.next(), parts.next()) {
            let n: f64 = n.parse().unwrap();
            v = match op {
                "\u{d7}" => v * n,
                "\u{f7}" => v / n,
                _ => panic!("{op} in {sum}"),
            };
        }
        v
    }

    #[std::prelude::v1::test]
    fn the_guides_corrections_show_working_from_its_own_words() {
        let note = first_run_note(Mode::ThinksWithYou);
        let joined = note.joined_text();
        let checks: Vec<(&Thread, &Check)> =
            note.threads.iter().filter_map(|t| Some((t, t.messages.iter().find_map(|m| m.check.as_ref())?))).collect();
        assert_eq!(checks.len(), 2, "both corrections carry a check");
        // the sum: every result right, every number the guide's own (from the corrected
        // paragraph) or a unit, and every given linked to words on the page
        let (t, c) = checks.iter().find(|(_, c)| !c.sum.is_empty()).unwrap();
        let paragraph = note.blocks.iter().find(|b| b.text.contains(&t.quote)).unwrap();
        for line in &c.sum {
            assert_eq!(work_out(&line.sum), line.is.parse::<f64>().unwrap(), "{}", line.sum);
            for n in line.sum.split_whitespace().filter(|w| w.parse::<f64>().is_ok()) {
                assert!(n == "1000" || c.givens.iter().any(|g| g.value == n), "{n} is neither a given nor a unit");
            }
        }
        for g in &c.givens {
            assert!(paragraph.text.contains(&g.words) && g.words.starts_with(&g.value), "{g:?}");
            assert!(matches!(crate::working::place(&joined, None, &g.words), crate::working::Place::Note(_)), "{g:?}");
        }
        // your own words: elsewhere in the note, not the corrected words
        let (t, c) = checks.iter().find(|(_, c)| !c.words.is_empty()).unwrap();
        for p in &c.words {
            assert!(matches!(crate::working::place(&joined, None, &p.words), crate::working::Place::Note(_)), "{p:?}");
            assert!(!t.quote.contains(&p.words) && !p.words.contains(&t.quote));
        }
    }

    #[std::prelude::v1::test]
    fn one_point_is_let_stand_and_one_thread_is_yours() {
        let note = first_run_note(Mode::ThinksWithYou);
        let stood: Vec<_> = note.threads.iter().filter(|t| t.ending() == crate::store::Ending::LetStand).collect();
        assert_eq!(stood.len(), 1);
        assert!(stood[0].is_point() && stood[0].reason.as_deref().is_some_and(|r| !r.is_empty()) && stood[0].ended_at.is_some());
        let own: Vec<_> = note.threads.iter().filter(|t| t.own).collect();
        assert_eq!(own.len(), 1);
        assert!(own[0].mind_name.is_empty() && own[0].status == ThreadStatus::Open && own[0].asker() == "You");
        assert!(own[0].messages.len() == 1 && own[0].messages[0].role == Role::You);
    }
}
