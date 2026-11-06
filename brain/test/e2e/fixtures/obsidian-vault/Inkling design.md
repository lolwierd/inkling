---
tags: [inkling, design]
aliases: [Loose Threads]
created: 2026-10-30
---
# Inkling design

Inkling reads along as you write and pulls **one** question into the margin, never a _wall_ of them. ^intro

The look is [[Loose Threads]]; the data lives in [[Store#Notes|the store]], one JSON file per note.
![[margin sketch.png]]

The reading loop waits for a pause,
then reads the paragraph you left.

## What it reads
- the paragraph you just finished
- a sentence that starts with `Inkling,`
- [[Pace|your pace]], slow or regular
	- nested items lose their indent

1. Pause.
2. Read.
3. Ask, or stay quiet.

> Quiet is the default.
> A question has to earn its thread.

> [!tip] Why imported notes stay quiet
> Imported words are left alone, so a vault costs no model calls.

%% a comment Obsidian hides %%
Notes on [the design doc](https://example.com/design_doc) live elsewhere.

```rust
fn main() {

    println!("**not bold**, [[not a link]]");
}
```

### Open questions
- [ ] Should tags become folders?
- [x] Strip front matter
