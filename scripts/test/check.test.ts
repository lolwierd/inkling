import { expect, test } from "bun:test";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

for (const suiteCode of [0, 1]) {
  test(`check --e2e --json emits one report when the suite exits ${suiteCode}`, async () => {
    const dir = mkdtempSync(join(tmpdir(), "inkling-check-test-"));
    try {
      for (const name of ["cargo", "bun"]) {
        const file = join(dir, name);
        writeFileSync(file, "#!/bin/sh\necho '1 pass'\nexit 0\n");
        chmodSync(file, 0o755);
      }
      const preload = join(dir, "suite.ts");
      const module = resolve(import.meta.dir, "../lib/e2e.ts");
      writeFileSync(preload, `import { mock } from "bun:test";
mock.module(${JSON.stringify(module)}, () => ({
  compactDump: () => ({}), listScenarios: () => [],
  runSuite: async (options) => {
    if (options.summary !== false) console.log(JSON.stringify({ extraSummary: true }));
    return ${suiteCode};
  },
}));`);
      const child = Bun.spawn([process.execPath, "--preload", preload, "scripts/dev", "check", "--e2e", "--json"], {
        cwd: resolve(import.meta.dir, "../.."),
        env: { ...process.env, PATH: `${dir}:${process.env.PATH}` }, stdout: "pipe", stderr: "pipe",
      });
      const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
      expect(stderr).toBe("");
      expect(code).toBe(suiteCode);
      const report = JSON.parse(stdout);
      expect(report.ok).toBe(suiteCode === 0);
      expect(report.e2e).toBe(suiteCode === 0);
      expect(report.e2e_status).toBe(suiteCode === 0 ? "pass" : "fail");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
}

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
