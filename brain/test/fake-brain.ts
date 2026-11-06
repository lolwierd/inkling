// The real brain (src/main.ts) with the SDK swapped for test/fake-sdk.ts, so the
// protocol can be driven end to end without a model. The Rust client's tests
// run it through INKLING_BRAIN_ENTRY.
import { setQueryForTests } from "../src/claude.ts";
import { setPageFetchForTests } from "../src/web.ts";
import { fakePages, fakeQuery } from "./fake-sdk.ts";

setQueryForTests(fakeQuery as any);
// No network in tests: the brain opens only the pages a directive serves.
setPageFetchForTests(async (url) => fakePages.get(url) ?? null);
await import("../src/main.ts");
