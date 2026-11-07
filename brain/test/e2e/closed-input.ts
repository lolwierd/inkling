// Keep stdout alive after closing stdin, so the writer failure arrives before EOF.
import { closeSync, readSync, writeSync } from "node:fs";
// Consume startup status first, so even a fast child cannot fail the startup write
// and force the app to restart it before the scenario's request reaches this process.
const byte = Buffer.alloc(1);
while (readSync(0, byte, 0, 1, null) === 1 && byte[0] !== 10) {}
writeSync(2, "Error: closed input fixture\n");
closeSync(0);
writeSync(1, '{"ev":"ready"}\n');
setInterval(() => {}, 1000);
