# Robustness changes, November 2026

This work started from the code-health audit of `cfb62f0`, reproduced its failures on
`675b12a`, and was rebased onto the folder/menu changes in `3279484`. The owner’s
notebook and the real model were never used. Regression scenarios use isolated
notebooks and the fake brain.

## Ownership and transitions

`note_page/lifecycle.rs` owns request bookkeeping and background profile learning.
The page still owns presentation state; this change does not claim to have turned
the entire page into a single state machine.

| Operation | Start | Finish | Cancellation or stale result |
|---|---|---|---|
| Read | Hold the page’s read slot | Record the completed hash, apply the result, release the slot | Leave the paragraph unread |
| Call by name | Track an in-flight hash to avoid duplicate calls | Record the hash after a nonempty answer | Release the hash without recording completion |
| Thread recheck | Capture the quoted words; wait if the thread is busy | Apply only if the anchor still holds those exact words | Queue the latest edit; do not resolve newer words with an older answer |
| Reply | Use the thread’s author; capture its current words | Append the reply; resolve only when the words are still current | An interrupted request cannot subsequently resolve or append |
| Note switch | Increment the visit generation, cancel requests, save, reset note-local caches | Load the next note | Deferred effects from an older visit are ignored, including an A → B → A round trip |
| Learning | Queue messages; send one with the latest profile and revision | Commit only if consent and revision still agree, then start the next | Opt-out, an off/on cycle or a manual edit invalidates old work |

The request map owns each timeout task. Removing the entry cancels the timer and
releases the captured callback and request payload immediately. Previously those
payloads survived until the detached 120/600-second timeout expired.

## Saving and resource limits

Failed saves of edits, settings, folders and minds retain the newest serialized
value per file. A later successful save replaces that pending value. Linked-note
saves retry even when the in-memory thread already matches. The window checks
the retry ledger every five seconds and surfaces the first failure for each file.
The ledger is in memory: it is not a second on-disk journal, and cannot preserve
unsaved data through a forced process termination or a persistently unwritable disk.

Import writes remain explicit: a failed note is counted as unsaved and can be
imported again. It is not inserted into the in-memory notebook or counted as a
successful import. A failed deletion keeps the note in memory; successful deletion
also removes a queued older save so retry cannot bring it back.

The sidecar has a dedicated input writer with a bounded queue of 32 requests. Each
wire request is capped at 2 MiB, and overload is returned as an error without
blocking the UI. Cancellation kills an unresponsive process if its input cannot
accept the cancellation. Shutdown attempts a graceful exit for at most 300 ms,
then kills and reaps the process. The OS/process scheduler can add latency to that
deadline; it is not a real-time guarantee.

The real brain rejects a task whose serialized input exceeds 256 KiB before
calling the SDK. It reports that the note/conversation is too large rather than
silently removing context. This deliberately replaces the old test that forwarded
a 5.4 MB note to a fake SDK. A roughly 208 KB Unicode-containing request is still
tested for intact delivery. This is a byte budget, not a token estimate or a claim
about any particular model’s context window.

Web source verification reads at most 4 MiB from the response stream, including
chunked/decompressed responses, and cancels an oversized body. Undo and redo each
retain at most 300 snapshots and roughly 32 MiB of snapshot-owned data. The nearest
single snapshot is retained even when it alone exceeds the budget, preserving one
undo/redo step for a very large note. This does not bound the entire app’s memory.

## Performance measurements

These are local microbenchmarks in the optimized debug test build, run serially.
They measure the affected algorithms, not end-to-end frame times. Timing assertions
are avoided for the scaling measurements; the pipe regression allows ample timing
slack and uses a child that does not read stdin.

| Reproduction | Audit baseline | After |
|---|---:|---:|
| Code-unit discovery across a 20,000-line code block | 200.621 ms | 0.010 ms |
| Open-thread counts for 10,000 empty notes | 200.633 ms | 0.036 ms |
| Enqueue a 1 MB payload to a nonreading child | 1,016.2 ms | 0.3 ms |
| Retained snapshot text after 300 edits to a 1 MB note | 300,046,050 bytes | 33,009,471 bytes |

Gutter rendering advances over code-block interiors, indexes anchors once, uses
binary search over ordered question positions, and builds one read-hash set per
frame. Sidebar and folder rows copy only display fields instead of whole notes,
and their counts are computed once. Sidebar grouping is one pass over notes.
Font size and line metrics now participate in layout reuse. Mind names participate
in passage-index invalidation.

## Audit disposition

| Audit IDs | Disposition |
|---|---|
| H01, H02, H06, H20–H25 | Reproduced correctness failures addressed by the changes above and regression coverage |
| H03–H05, H07 | Measured hot paths/resource growth addressed; timings above are algorithm measurements |
| H08 | Failed writes retained/reported, failed deletion preserved, stale retries cannot resurrect deletions |
| H11 | Bounded by an explicit pre-model request limit; contextual retrieval remains future work |
| H12, H13 | Stream size bound and mind-name index invalidation addressed |
| H14–H16, H26 | Harness log filtering, skipped status, JSON output, exit codes and invalid tag-toggle scenario corrected |
| H17 | Request lifetime and learning ownership extracted; broader page presentation state remains distributed |
| H19 | Added race, persistence, memory, stream and harness coverage; `scripts/dev check` includes the harness tests and is used by CI |
| H09, H10, H18, H27 | Remaining structural/performance work, described below |

## Remaining work and limits

Persistence and import serialization still run synchronously. Moving them off the
UI thread needs one ordered writer with explicit flush/delete barriers and a
revision check. Spawning unrelated write tasks would reintroduce stale writes and
shutdown races. The current changes favor correct save ordering; they do not claim
to remove disk-related UI stalls or provide multi-file transactions.

The margin still builds/clones all open threads, and most lists are not virtualized.
Layout still tokenizes code across the document when its cache is invalidated.
Read-it-through reconstructs remaining units between model calls. The gutter fixes
do not eliminate those separate costs.

Large rendering and dispatch functions remain. Cyclomatic complexity is useful
for choosing where to inspect, but splitting a match statement just to reduce its
score would not strengthen a runtime invariant. Further extraction should follow
an ownership boundary with a regression that demonstrates the benefit.

No real-model accuracy, live SDK subscription behavior, power-loss recovery or
production CI run is claimed by these local tests. A fake brain makes event
ordering reproducible; it cannot make model answers deterministic.

## Reproduce validation

Use `scripts/dev check`, then `scripts/dev e2e` for the complete app suite. The new
race scenarios are `r18` through `r28`; `u33` also exercises the concurrently added
folder/menu functionality. `cargo test robustness_tests -- --test-threads=1 --nocapture`
runs the persistence/layout/pipe tests and prints the algorithm measurements.

Screen captures require an unlocked macOS desktop and Screen Recording permission.
A capture failure must not be called a visual pass even when every state assertion
in the same scenario passes. Preserve the run directory and distinguish a
behavioral failure from a blocked screenshot in the validation report.
