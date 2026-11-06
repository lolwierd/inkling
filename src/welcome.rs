//! The first note a new notebook opens with. It shows a margin thread and a
//! drawing straight away, so the idea lands before any model call.

use chrono::Utc;

use crate::store::{
    Block, BlockKind, Message, Mode, Note, Role, Side, Thread, ThoughtKind, ThreadStatus, new_id,
};

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
    });
    for b in note.blocks.iter().skip(1) {
        note.read_hashes.push(crate::store::hash_text(&b.text));
    }
    note
}

/// The first note of a new notebook. Instead of a ready-made example it asks for one
/// sentence about something you are working on; the first thing Inkling asks is about
/// that sentence, so the idea lands on your own words.
pub fn first_run_note(mode: Mode) -> Note {
    let mut note = Note::new("Start here", mode, None);
    let intro = "Write one sentence below about something you\u{2019}re working on, or something you believe. Inkling will read it and ask you about it.";
    note.blocks = vec![
        Block { kind: BlockKind::Title, text: "Start here".into() },
        Block { kind: BlockKind::Paragraph, text: intro.into() },
        Block { kind: BlockKind::Paragraph, text: String::new() },
    ];
    note.read_hashes = vec![crate::store::hash_text(intro)];
    note.first_run = true;
    note
}
