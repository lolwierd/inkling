// The real brain (src/main.ts) with the SDK swapped for test/fake-sdk.ts, so the
// protocol can be driven end to end without a model. The Rust client's tests
// run it through INKLING_BRAIN_ENTRY.
import { setQueryForTests } from "../src/claude.ts";
import { fakeQuery } from "./fake-sdk.ts";

setQueryForTests(fakeQuery as any);
await import("../src/main.ts");
