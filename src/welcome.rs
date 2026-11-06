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
        (P, "A \u{d7} on a tag is a correction and a ? is a question. Open questions are numbered in reading order, with a knot beside their paragraph, and \u{2325}\u{2318}\u{2193} jumps to the next one. Fix the words a thread is about and Inkling reads them again; if the problem is gone, the thread ties itself off and its knot joins the border at the top of the page."),
        (P, "Open a tag to write back, and the mind answers in the same thread. Resolve ties it off. Dismiss snips it, and Undo or \u{2325}\u{2318}Z brings it back. Copy takes the whole thread with you, and Make a note turns it into a note of its own."),
        (H2, "Reading along"),
        (P, "Inkling reads each paragraph when you pause, and a small stitch in the gutter marks what it has read. Most of the time it says nothing. A question that comes while you are mid-sentence waits, folded, until you stop."),
        (P, "Tell Inkling, above the title, says what a note is for, and every read takes it into account. Beside it, Reads along can become Waits to be asked: then Inkling stays quiet until you call it, select words, or open the chat."),
        (H2, "Asking"),
        (P, "Select some words and a small menu appears. Find a loose thread (\u{2318}\u{21b5}) picks the one thing in them most worth a question. Ask Inkling takes your own question. Sketch it answers with a drawing, stitched out step by step; click a sketch to see it large. The same menu turns a line into a heading, a list or a quote."),
        (P, enough),
        (P, "Ask the room to hear from up to three minds at once. Each answers on its own thread, in its own colour. Or start a sentence with a mind\u{2019}s name and a comma, and that mind answers you there."),
        (P, moon),
        (P, "\u{2318}J opens a chat about the whole note. It has read every thread as well, so it can tell you what you have settled and what is still open."),
        (H2, "Minds"),
        (P, "The name above the title is the note\u{2019}s mind. Inkling comes with Staff engineer, Editor, Skeptic, Socratic tutor and Investor. In Settings, under Minds, you can make your own: say what it is for and how it thinks, give it a voice (plain, warm, blunt or Socratic), and choose how often it speaks. Each mind keeps its own thread colour, so you can see who asked what."),
        (P, "Settings, under About you, holds what Inkling knows about you. Turn on Inkling learns about you, and it keeps what matters from your replies."),
        (H2, "Your notebook"),
        (P, "\u{2318}\\ opens your notes and folders; a dot marks notes with open threads. \u{2318}K finds a note or starts one. \u{2318}\u{21e7}C copies a note, and \u{2318}\u{21e7}E exports it as Markdown, HTML or Word, with its threads or without."),
        (H2, "Make it yours"),
        (P, "In Settings (\u{2318},) choose Light, Dark or the system\u{2019}s look, the text size, the lettering, and your thread colour: madder, indigo, sage, rose, plum, walnut or ochre. Questions can appear in both margins, in the right margin only, or as small marks in the text. Reading pace can be regular or slow. Ink from the pointer lets a drop of ink fall now and then as you move it."),
        (P, "Hold \u{2325} on its own to speak instead of typing. Under Model, pick which Claude model Inkling uses, and how much effort it spends reading along and answering. It runs on the Claude login already on this Mac."),
        (H2, "Quiet when you want it"),
        (P, "The window\u{2019}s controls fade while you write and come back when you move the pointer. \u{2318}\u{21e7}. hides every thread and leaves just your words."),
        (H2, "Keys"),
        (Bullet, "\u{2318}\u{21b5} finds a loose thread in what you selected."),
        (Bullet, "\u{2325}\u{2318}\u{2193} and \u{2325}\u{2318}\u{2191} move between open threads."),
        (Bullet, "\u{2318}\u{21e7}\u{232b} dismisses the open thread, and \u{2325}\u{2318}Z brings it back."),
        (Bullet, "\u{2318}\u{21e7}. hides every thread and leaves just your words."),
        (Bullet, "\u{2318}J opens a chat about the whole note."),
        (Bullet, "\u{2318}K finds a note, or starts one."),
        (Bullet, "\u{2318}\\ shows your notes and folders."),
        (Bullet, "\u{2318}\u{21e7}C copies the note, and \u{2318}\u{21e7}E exports it."),
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
