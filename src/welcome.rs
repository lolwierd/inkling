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

/// The first note of a new notebook, and the Welcome Guide in the Help menu: a short
/// tour that is itself a working note. Its threads are real (point at them, answer
/// them, fix the words and watch one tie off), and it ends by asking for one sentence
/// of your own, which Inkling reads with a question first (`first_run`).
pub fn first_run_note(mode: Mode) -> Note {
    use BlockKind::{Bullet, H2, Paragraph as P, Title};
    let mut note = Note::new("Start here", mode, None);
    let pace = "A 10 km race at 5:30 per kilometre takes 50 minutes.";
    let memory = "The best way to remember something is to write it down.";
    let enough = "Four runs a week is enough to get faster.";
    let moon = "The far side of the Moon is dark because sunlight never reaches it.";
    let blocks: Vec<(BlockKind, &str)> = vec![
        (Title, "Start here"),
        (P, "Inkling is a notebook that asks. You write, and it reads along. When a line doesn\u{2019}t add up, it pulls a thread from those words into the margin: a question, or a correction. It never writes your notes for you. The thinking stays yours."),
        (P, "Everything on this page works. Point at a tag in the margin to read it, click it to answer, and drag it anywhere you like."),
        (H2, "Threads"),
        (P, pace),
        (P, memory),
        (P, "A \u{d7} on a tag is a correction and a ? is a question. Open questions are numbered in reading order, with a knot beside their paragraph. \u{2325}\u{2318}\u{2193} jumps to the next one. Fix the words a thread is about and Inkling reads them again; if the problem is gone, the thread ties itself off and its knot joins the border at the top of the page."),
        (H2, "Asking"),
        (P, "Select some words and a small menu appears. Find a loose thread (\u{2318}\u{21b5}) picks the one thing in them most worth a question. Ask Inkling takes your own question. Sketch it answers with a drawing, stitched out step by step."),
        (P, enough),
        (P, "Ask the room to hear from up to three minds at once. Each answers on its own thread, in its own colour. Or start a sentence with a mind\u{2019}s name and a comma, and that mind answers you there."),
        (P, moon),
        (H2, "Quiet when you want it"),
        (P, "Inkling waits while you type and reads at your pauses. Tell Inkling, above the title, says what a note is for. Switch Reads along to Waits to be asked and it answers only when you ask. The window\u{2019}s controls fade while you write and come back when you move the pointer."),
        (H2, "Keys"),
        (Bullet, "\u{2318}\u{21b5} finds a loose thread in what you selected."),
        (Bullet, "\u{2325}\u{2318}\u{2193} and \u{2325}\u{2318}\u{2191} move between open threads."),
        (Bullet, "\u{2318}\u{21e7}\u{232b} dismisses the open thread, and \u{2325}\u{2318}Z brings it back."),
        (Bullet, "\u{2318}\u{21e7}. hides every thread and leaves just your words."),
        (Bullet, "\u{2318}J opens a chat about the whole note."),
        (Bullet, "\u{2318}K finds a note, or starts one."),
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
        }
    };
    let mind = |text: &str| Message::new(Role::Mind, text);
    let you = |text: &str| Message::new(Role::You, text);
    note.threads = vec![
        thread(
            "takes 50 minutes",
            ThoughtKind::Correction,
            Side::Right,
            "Inkling",
            vec![mind("At 5:30 a kilometre, 10 km takes 55 minutes. Finishing in 50 needs 5:00 a kilometre. Change the number in the sentence and this thread ties itself off.")],
            None,
        ),
        thread(
            memory,
            ThoughtKind::Question,
            Side::Left,
            "Inkling",
            vec![mind("Better than what? Recalling it with the page closed tends to stick better than writing it out again. Which have you tried?")],
            None,
        ),
        thread(
            enough,
            ThoughtKind::Ask,
            Side::Right,
            "Skeptic",
            vec![
                you("Is this enough?"),
                mind("Enough for what, and compared to what? Four easy runs keep you fit; one hard session among them is what usually makes you faster. What does your week look like now?"),
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
                mind("What has to change in your legs for 5:00 a kilometre to feel like 5:30 does today? Which of your four runs works on that?"),
            ],
            None,
        ),
        thread(
            moon,
            ThoughtKind::Correction,
            Side::Left,
            "Inkling",
            vec![mind("It gets as much sunlight as the side we see. It is only hidden from us: the Moon turns once per orbit, so one face always points at Earth. At new Moon, which side is lit?")],
            Some(MOON_SKETCH),
        ),
    ];
    note.first_run = true;
    note
}
