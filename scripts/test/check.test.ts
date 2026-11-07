import { expect, test } from "bun:test";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

test("a failed prerequisite cannot report the unrun e2e suite as passed", async () => {
  const dir = mkdtempSync(join(tmpdir(), "inkling-check-test-"));
  try {
    // Exercise the real CLI/reporting path, without a compiler or a GUI launch.
    for (const [name, body] of [["cargo", "exit 1"], ["bun", "echo '1 pass'; exit 0"]]) {
      const file = join(dir, name);
      writeFileSync(file, `#!/bin/sh\n${body}\n`);
      chmodSync(file, 0o755);
    }
    const child = Bun.spawn([process.execPath, "scripts/dev", "check", "--e2e", "--json"], {
      cwd: resolve(import.meta.dir, "../.."),
      env: { ...process.env, PATH: `${dir}:${process.env.PATH}` },
      stdout: "pipe", stderr: "pipe",
    });
    const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    expect(code).toBe(1);
    const result = JSON.parse(stdout);
    expect(result.ok).toBe(false);
    expect(result.e2e).toBe(null);
    expect(result.e2e_status).toBe("skipped");
    expect(stderr).not.toContain("e2e:");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
