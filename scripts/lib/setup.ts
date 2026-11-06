// Getting a checkout ready to work in, and saying what is missing when it is not.
// Agents work in git worktrees under .worktrees/; a fresh one has no target/ (a cold
// build takes minutes) and no brain/node_modules (every brain test fails). Both can be
// cloned from the main checkout with APFS copy-on-write: seconds, no extra disk.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { REPO, binary, outRoot, sh } from "./app.ts";

export async function mainCheckout(): Promise<string> {
  const r = await sh(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"]);
  return dirname(r.out.trim());
}

/** What this checkout lacks for `scripts/dev` to work, with the fix for each. */
export async function missing(): Promise<string[]> {
  const out: string[] = [];
  const main = await mainCheckout();
  const clone = (what: string) =>
    main !== REPO && existsSync(join(main, what)) ? `cp -c -R ${join(main, what)} ${join(REPO, what)}` : null;
  if (!existsSync(join(REPO, "brain/node_modules"))) {
    out.push(`brain/node_modules is missing: ${clone("brain/node_modules") ?? "cd brain && bun install"}`);
  }
  if (!existsSync(binary()) && !existsSync(join(REPO, "target"))) {
    const c = clone("target");
    if (c) out.push(`target/ is missing, so the first build is cold (minutes). Clone the main checkout's: ${c}`);
  }
  return out;
}

/** `scripts/dev worktree NAME`: a new branch and worktree under .worktrees/NAME, with
 *  the main checkout's build and brain dependencies cloned in. */
export async function makeWorktree(name: string): Promise<number> {
  if (!/^[\w.-]+$/.test(name)) {
    console.error(`a worktree name is letters, digits, dot, dash or underscore: ${name}`);
    return 2;
  }
  const main = await mainCheckout();
  const dir = join(main, ".worktrees", name);
  if (existsSync(dir)) {
    console.error(`${dir} already exists`);
    return 2;
  }
  const add = await sh(["git", "-C", main, "worktree", "add", dir, "-b", name]);
  if (add.code !== 0) {
    console.error(add.err.trim());
    return 1;
  }
  for (const what of ["target", "brain/node_modules"]) {
    if (!existsSync(join(main, what))) continue;
    const r = await sh(["cp", "-c", "-R", join(main, what), join(dir, what)]);
    console.log(r.code === 0 ? `cloned ${what} (${(r.ms / 1000).toFixed(1)} s)` : `could not clone ${what}: ${r.err.trim()}`);
  }
  console.log(`worktree ready: ${dir} (branch ${name}). Commit early: uncommitted work in a worktree has been lost before.`);
  return 0;
}

/** `scripts/dev doctor`: the environment, and anything that would bite. */
export async function doctor(): Promise<number> {
  const lines: [string, boolean, string][] = [];
  const tool = async (name: string, cmd: string[]) => {
    const r = await sh(cmd).catch(() => ({ code: 1, out: "", err: "not found" }));
    lines.push([name, r.code === 0, (r.out || r.err).trim().split("\n")[0]]);
  };
  await tool("bun", ["bun", "--version"]);
  await tool("cargo", ["cargo", "--version"]);
  for (const m of await missing()) lines.push(["checkout", false, m]);
  lines.push(["app binary", existsSync(binary()), existsSync(binary()) ? binary() : "not built yet (scripts/dev builds it)"]);
  const real = join(process.env.HOME ?? "", "Library/Application Support/Inkling");
  const dataDir = process.env.INKLING_DATA_DIR;
  if (dataDir && dataDir.replace(/\/$/, "") === real) lines.push(["INKLING_DATA_DIR", false, "points at the owner's real notebook; unset it"]);
  const lock = join(outRoot(), "e2e.lock", "owner.json");
  if (existsSync(lock)) lines.push(["e2e lock", true, `held: ${readFileSync(lock, "utf8").trim()}`]);
  const apps = await sh(["pgrep", "-fl", "MacOS/inkling|debug/inkling"]);
  lines.push(["running apps", true, apps.out.trim() ? apps.out.trim().split("\n").length + " Inkling process(es) on this machine (other agents or the owner)" : "none"]);
  lines.push(["runs kept in", true, outRoot()]);
  for (const [name, ok, detail] of lines) console.log(`${ok ? "ok  " : "FIX "} ${name.padEnd(16)} ${detail}`);
  return lines.every(([, ok]) => ok) ? 0 : 1;
}
