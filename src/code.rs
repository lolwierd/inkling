//! Code in a note: which language a fence names, a small lexer for each language it
//! knows, and the thread each kind of token is set in.
//!
//! Code is the one place a note is allowed a grid: monospace on a flat ground. Its
//! colours come from the thread box (indigo, sage, ochre, plum, walnut), two greys and
//! ink. Madder and rose never: a madder mark in a code block always means Inkling.
//! Every colour reads at 4.5:1 or better on the code ground, in both lights (tested).
//!
//! The lexer is deliberately small: keywords, strings, numbers, comments, types,
//! macros and attributes, punctuation. A block comment or a string that runs on past
//! the end of a line carries over to the next line of the block.

use std::ops::Range;

use gpui_kit::{FontFeatures, Hsla, Rgba};

use crate::store::{InkColor, Lang};
use crate::theme::{Palette, thread_hex};

/// What a piece of code is, which decides its colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Keyword,
    String,
    Number,
    Comment,
    Type,
    /// Macros (`println!`), attributes (`#[test]`), decorators (`@cache`), shell variables.
    Macro,
    Punct,
}

impl Token {
    pub const ALL: [Token; 7] = [Token::Keyword, Token::String, Token::Number, Token::Comment, Token::Type, Token::Macro, Token::Punct];
}

/// The colour of a token on the code ground: indigo keywords, sage strings and ochre
/// numbers (both taken 12% towards ink, so they read at 4.5:1), plum types, walnut
/// macros, faint comments, graphite punctuation. Everything else is ink.
pub fn color(token: Token, p: &Palette) -> Hsla {
    let thread = |ink: InkColor| -> u32 {
        let (light, dark) = thread_hex(ink);
        if p.dark { dark } else { light }
    };
    match token {
        Token::Keyword => hex(thread(InkColor::Blue)),
        Token::String => toward(hex(thread(InkColor::Green)), p.ink, 0.12),
        Token::Number => toward(hex(thread(InkColor::Orange)), p.ink, 0.12),
        Token::Type => hex(thread(InkColor::Purple)),
        Token::Macro => hex(thread(InkColor::Brown)),
        Token::Comment => p.faint,
        Token::Punct => p.graphite,
    }
}

fn hex(v: u32) -> Hsla {
    gpui_kit::rgb(v).into()
}

/// `a` taken `t` of the way to `b`, mixed in sRGB as CSS `color-mix(in srgb)` does.
fn toward(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.to_rgb(), b.to_rgb());
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1. }.into()
}

/// The dotted zero on, contextual alternates off, for every run of code.
pub fn features() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![("zero".into(), 1), ("calt".into(), 0)]))
}

// ----- languages ---------------------------------------------------------------------

/// How one language is written, as far as colour goes.
struct Syntax {
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    line_comments: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
    /// Characters that open a string.
    quotes: &'static str,
    /// Of `quotes`, the ones whose strings may run on to the next line.
    multiline: &'static str,
    /// Python's `"""` and `'''`.
    triple: bool,
    /// An identifier starting with a capital is a type.
    capitals: bool,
    /// `name!` is a macro, `#[…]` an attribute and `'a` a lifetime (Rust).
    rust: bool,
    /// `@name` is a decorator or annotation.
    at: bool,
    /// `$name` and `${…}` are variables (shells).
    dollar: bool,
    /// Keywords match in any case (SQL).
    any_case: bool,
}

const BASE: Syntax = Syntax {
    keywords: &[],
    types: &[],
    line_comments: &[],
    block_comment: None,
    quotes: "\"'",
    multiline: "",
    triple: false,
    capitals: false,
    rust: false,
    at: false,
    dollar: false,
    any_case: false,
};

const C_COMMENTS: (&str, &str) = ("/*", "*/");

const RUST: Syntax = Syntax {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
        "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "type",
        "unsafe", "use", "where", "while", "yield",
    ],
    types: &[
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64", "bool", "char", "str", "Self",
    ],
    line_comments: &["//"],
    block_comment: Some(C_COMMENTS),
    quotes: "\"",
    multiline: "\"",
    capitals: true,
    rust: true,
    ..BASE
};

const JS: Syntax = Syntax {
    keywords: &[
        "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger", "declare", "default", "delete",
        "do", "else", "enum", "export", "extends", "false", "finally", "for", "from", "function", "get", "if", "implements", "import", "in",
        "infer", "instanceof", "interface", "is", "keyof", "let", "namespace", "new", "null", "of", "private", "protected", "public", "readonly",
        "return", "satisfies", "set", "static", "super", "switch", "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void",
        "while", "with", "yield",
    ],
    types: &["any", "bigint", "boolean", "never", "number", "object", "string", "symbol", "unknown"],
    line_comments: &["//"],
    block_comment: Some(C_COMMENTS),
    quotes: "\"'`",
    multiline: "`",
    capitals: true,
    at: true,
    ..BASE
};

const PYTHON: Syntax = Syntax {
    keywords: &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "case", "class", "continue", "def", "del", "elif", "else",
        "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise",
        "return", "self", "try", "while", "with", "yield",
    ],
    types: &["bool", "bytes", "dict", "float", "frozenset", "int", "list", "object", "set", "str", "tuple"],
    line_comments: &["#"],
    triple: true,
    capitals: true,
    at: true,
    ..BASE
};

const GO: Syntax = Syntax {
    keywords: &[
        "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "false", "for", "func", "go", "goto", "if",
        "import", "interface", "iota", "map", "nil", "package", "range", "return", "select", "struct", "switch", "true", "type", "var",
    ],
    types: &[
        "any", "bool", "byte", "complex64", "complex128", "error", "float32", "float64", "int", "int8", "int16", "int32", "int64", "rune", "string",
        "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
    ],
    line_comments: &["//"],
    block_comment: Some(C_COMMENTS),
    quotes: "\"'`",
    multiline: "`",
    ..BASE
};

/// C, C++, Java, C#, Kotlin and Swift share their shape; the words are the union.
const C_LIKE: Syntax = Syntax {
    keywords: &[
        "abstract", "async", "await", "break", "case", "catch", "class", "const", "constexpr", "continue", "data", "default", "defer", "delete",
        "do", "else", "enum", "extends", "extension", "false", "final", "finally", "for", "fun", "func", "goto", "guard", "if", "implements",
        "import", "in", "inline", "interface", "internal", "is", "let", "namespace", "new", "nil", "null", "nullptr", "object", "operator",
        "override", "package", "private", "protected", "protocol", "public", "return", "sealed", "self", "sizeof", "static", "struct", "super",
        "switch", "template", "this", "throw", "throws", "true", "try", "typedef", "typename", "union", "using", "val", "var", "virtual",
        "volatile", "when", "where", "while",
    ],
    types: &[
        "auto", "bool", "boolean", "byte", "char", "double", "float", "int", "long", "short", "signed", "size_t", "string", "uint8_t", "uint16_t",
        "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t", "int64_t", "unsigned", "void",
    ],
    line_comments: &["//"],
    block_comment: Some(C_COMMENTS),
    capitals: true,
    at: true,
    ..BASE
};

const SHELL: Syntax = Syntax {
    keywords: &[
        "alias", "break", "case", "cd", "continue", "declare", "do", "done", "echo", "elif", "else", "esac", "exit", "export", "fi", "for",
        "function", "if", "in", "local", "readonly", "return", "set", "shift", "source", "then", "unset", "until", "while",
    ],
    line_comments: &["#"],
    multiline: "\"'",
    dollar: true,
    ..BASE
};

const SQL: Syntax = Syntax {
    keywords: &[
        "add", "all", "alter", "and", "as", "asc", "begin", "between", "by", "case", "check", "column", "commit", "constraint", "create",
        "default", "delete", "desc", "distinct", "drop", "else", "end", "exists", "false", "foreign", "from", "full", "group", "having", "if",
        "in", "index", "inner", "insert", "into", "is", "join", "key", "left", "like", "limit", "not", "null", "offset", "on", "or", "order",
        "outer", "primary", "references", "returning", "right", "rollback", "select", "set", "table", "then", "true", "union", "unique",
        "update", "values", "view", "when", "where", "with",
    ],
    types: &[
        "bigint", "bool", "boolean", "char", "date", "decimal", "double", "float", "int", "integer", "json", "jsonb", "numeric", "real",
        "serial", "smallint", "text", "time", "timestamp", "timestamptz", "uuid", "varchar",
    ],
    line_comments: &["--"],
    block_comment: Some(C_COMMENTS),
    quotes: "'\"",
    multiline: "'",
    any_case: true,
    ..BASE
};

const JSON: Syntax = Syntax { keywords: &["false", "null", "true"], line_comments: &["//"], block_comment: Some(C_COMMENTS), quotes: "\"", ..BASE };

const YAML: Syntax = Syntax { keywords: &["false", "no", "null", "off", "on", "true", "yes"], line_comments: &["#"], ..BASE };

const RUBY: Syntax = Syntax {
    keywords: &[
        "alias", "and", "begin", "break", "case", "class", "def", "defined?", "do", "else", "elsif", "end", "ensure", "false", "for", "if", "in",
        "module", "next", "nil", "not", "or", "redo", "rescue", "retry", "return", "self", "super", "then", "true", "undef", "unless", "until",
        "when", "while", "yield", "require", "attr_reader", "attr_accessor",
    ],
    line_comments: &["#"],
    capitals: true,
    ..BASE
};

const CSS: Syntax = Syntax {
    keywords: &["important", "inherit", "initial", "none", "unset"],
    block_comment: Some(C_COMMENTS),
    at: true,
    ..BASE
};

const LUA: Syntax = Syntax {
    keywords: &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat",
        "return", "then", "true", "until", "while",
    ],
    line_comments: &["--"],
    ..BASE
};

/// The syntax a fence's language names, by its usual names and file extensions.
fn syntax(lang: Lang) -> Option<&'static Syntax> {
    Some(match lang.as_str() {
        "rust" | "rs" => &RUST,
        "js" | "javascript" | "jsx" | "mjs" | "cjs" | "ts" | "typescript" | "tsx" | "mts" => &JS,
        "py" | "python" | "python3" | "py3" => &PYTHON,
        "go" | "golang" => &GO,
        "c" | "h" | "cpp" | "c++" | "cc" | "cxx" | "hpp" | "java" | "cs" | "csharp" | "c#" | "kotlin" | "kt" | "kts" | "swift" | "objc"
        | "objective-c" | "scala" | "dart" => &C_LIKE,
        "sh" | "bash" | "zsh" | "shell" | "console" | "shellsession" | "fish" => &SHELL,
        "sql" | "psql" | "postgres" | "postgresql" | "mysql" | "sqlite" => &SQL,
        "json" | "jsonc" | "json5" => &JSON,
        "yaml" | "yml" | "toml" | "ini" => &YAML,
        "rb" | "ruby" => &RUBY,
        "css" | "scss" | "less" => &CSS,
        "lua" => &LUA,
        _ => return None,
    })
}

/// Whether the lexer knows `lang` (anything else is set in plain ink).
pub fn knows(lang: Lang) -> bool {
    syntax(lang).is_some()
}

// ----- the lexer -----------------------------------------------------------------------

/// What runs on from one line of a block into the next.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Carry {
    #[default]
    Nothing,
    /// Inside a block comment.
    Comment,
    /// Inside a string opened by this quote; `triple` for Python's `"""`; `hashes` for
    /// a Rust raw string's `#`s (it ends at the quote and as many `#`s, and has no
    /// escapes).
    Str { quote: char, triple: bool, raw: bool, hashes: u8 },
}

/// The tokens of each line of one code block, with byte ranges inside each line.
pub fn lex_block(lang: Lang, lines: &[&str]) -> Vec<Vec<(Range<usize>, Token)>> {
    let Some(syn) = syntax(lang) else { return lines.iter().map(|_| vec![]).collect() };
    let mut carry = Carry::Nothing;
    lines
        .iter()
        .map(|line| {
            let (tokens, next) = lex_line(syn, line, carry);
            carry = next;
            tokens
        })
        .collect()
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// One line, starting in `carry`: its tokens and what runs on into the next line.
fn lex_line(syn: &Syntax, line: &str, mut carry: Carry) -> (Vec<(Range<usize>, Token)>, Carry) {
    let mut out: Vec<(Range<usize>, Token)> = vec![];
    let b = line.as_bytes();
    let n = line.len();
    let mut i = 0;
    let at = |i: usize, s: &str| line[i..].starts_with(s);
    let ch = |i: usize| line[i..].chars().next().unwrap_or('\0');
    while i < n {
        match carry {
            Carry::Comment => {
                let (_, close) = syn.block_comment.unwrap_or(C_COMMENTS);
                match line[i..].find(close) {
                    Some(k) => {
                        out.push((i..i + k + close.len(), Token::Comment));
                        i += k + close.len();
                        carry = Carry::Nothing;
                    }
                    None => {
                        out.push((i..n, Token::Comment));
                        return (out, carry);
                    }
                }
                continue;
            }
            Carry::Str { quote, triple, raw, hashes } => {
                let (end, closed) = string_end(line, i, quote, triple, raw, hashes);
                out.push((i..end, Token::String));
                i = end;
                if closed {
                    carry = Carry::Nothing;
                    continue;
                }
                return (out, carry);
            }
            Carry::Nothing => {}
        }
        let c = ch(i);
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if let Some(lc) = syn.line_comments.iter().find(|lc| at(i, lc)) {
            // `#` starts a comment only on its own or after a space in shells (`a#b` is a word)
            let starts = !(syn.dollar && *lc == "#" && i > 0 && !b[i - 1].is_ascii_whitespace());
            if starts {
                out.push((i..n, Token::Comment));
                return (out, Carry::Nothing);
            }
        }
        if let Some((open, close)) = syn.block_comment.filter(|(open, _)| at(i, open)) {
            let body = i + open.len();
            match line[body..].find(close) {
                Some(k) => {
                    out.push((i..body + k + close.len(), Token::Comment));
                    i = body + k + close.len();
                }
                None => {
                    out.push((i..n, Token::Comment));
                    return (out, Carry::Comment);
                }
            }
            continue;
        }
        if syn.rust && (at(i, "#[") || at(i, "#![")) {
            let end = line[i..].find(']').map_or(n, |k| i + k + 1);
            out.push((i..end, Token::Macro));
            i = end;
            continue;
        }
        if syn.at && c == '@' && line[i + 1..].chars().next().is_some_and(is_ident_start) {
            let end = ident_end(line, i + 1);
            out.push((i..end, Token::Macro));
            i = end;
            continue;
        }
        if syn.dollar && c == '$' {
            let end = if at(i, "${") {
                line[i..].find('}').map_or(n, |k| i + k + 1)
            } else {
                let next = line[i + 1..].chars().next();
                match next {
                    Some(d) if is_ident(d) => ident_end(line, i + 1),
                    Some(d) if "@#?$!*-".contains(d) => i + 2,
                    _ => i + 1,
                }
            };
            out.push((i..end, Token::Macro));
            i = end;
            continue;
        }
        // strings, with Python's prefixes (f"", rb'') and Rust's raw and byte strings
        if let Some((start_len, quote, triple, raw, hashes)) = string_start(syn, line, i) {
            let begin = i;
            i += start_len;
            let (end, closed) = string_end(line, i, quote, triple, raw, hashes);
            out.push((begin..end, Token::String));
            i = end;
            if !closed && (triple || raw || syn.multiline.contains(quote)) {
                return (out, Carry::Str { quote, triple, raw, hashes });
            }
            continue;
        }
        if syn.rust && c == '\'' {
            // a char ('x', '\n', '\u{2019}') or a lifetime ('a)
            if let Some(len) = rust_char(&line[i..]) {
                out.push((i..i + len, Token::String));
                i += len;
            } else {
                let end = ident_end(line, i + 1);
                out.push((i..end.max(i + 1), Token::Type));
                i = end.max(i + 1);
            }
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && line[i + 1..].starts_with(|d: char| d.is_ascii_digit()) && !(i > 0 && b[i - 1] == b'.')) {
            let mut end = i + 1;
            while end < n {
                let d = b[end] as char;
                // `1..10` is two numbers and a range
                if d == '.' && (b.get(end + 1) == Some(&b'.') || !b.get(end + 1).is_some_and(|x| x.is_ascii_digit())) {
                    break;
                }
                if d.is_ascii_alphanumeric() || d == '_' || d == '.' {
                    end += 1;
                } else {
                    break;
                }
            }
            out.push((i..end, Token::Number));
            i = end;
            continue;
        }
        if is_ident_start(c) {
            let end = ident_end(line, i);
            let word = &line[i..end];
            if syn.rust && line[end..].starts_with('!') && !line[end..].starts_with("!=") {
                out.push((i..end + 1, Token::Macro));
                i = end + 1;
                continue;
            }
            let is = |list: &[&str]| if syn.any_case { list.iter().any(|k| k.eq_ignore_ascii_case(word)) } else { list.contains(&word) };
            if is(syn.keywords) {
                out.push((i..end, Token::Keyword));
            } else if is(syn.types) || (syn.capitals && word.starts_with(|c: char| c.is_uppercase())) {
                out.push((i..end, Token::Type));
            }
            i = end;
            continue;
        }
        // anything else is punctuation: brackets, operators, separators
        out.push((i..i + c.len_utf8(), Token::Punct));
        i += c.len_utf8();
    }
    (out, carry)
}

fn ident_end(line: &str, from: usize) -> usize {
    line[from..].char_indices().find(|(_, c)| !is_ident(*c)).map_or(line.len(), |(k, _)| from + k)
}

/// A string opening at `i`: (length of the opening, quote, triple, raw, hashes).
fn string_start(syn: &Syntax, line: &str, i: usize) -> Option<(usize, char, bool, bool, u8)> {
    let rest = &line[i..];
    if syn.rust {
        // r"…", r#"…"#, br"…", b"…"
        let after_b = rest.strip_prefix('b').unwrap_or(rest);
        if let Some(r) = after_b.strip_prefix('r') {
            let hashes = r.len() - r.trim_start_matches('#').len();
            if r[hashes..].starts_with('"') && !(i > 0 && line[..i].ends_with(is_ident)) {
                return Some((rest.len() - r.len() + hashes + 1, '"', false, true, hashes.min(255) as u8));
            }
        }
        if after_b.len() < rest.len() && after_b.starts_with('"') && !(i > 0 && line[..i].ends_with(is_ident)) {
            return Some((2, '"', false, false, 0));
        }
    }
    if syn.triple {
        // Python's prefixes: f, r, b, u and pairs of them
        let prefix = rest.len() - rest.trim_start_matches(|c: char| "fFrRbBuU".contains(c)).len();
        if prefix <= 2 && !(i > 0 && line[..i].ends_with(is_ident)) {
            let q = &rest[prefix..];
            for quote in ['"', '\''] {
                let three: String = std::iter::repeat_n(quote, 3).collect();
                if q.starts_with(&three) {
                    return Some((prefix + 3, quote, true, rest[..prefix].contains(['r', 'R']), 0));
                }
            }
            if prefix > 0 && q.starts_with(['"', '\'']) {
                return Some((prefix + 1, q.chars().next().unwrap_or('"'), false, rest[..prefix].contains(['r', 'R']), 0));
            }
        }
    }
    let c = rest.chars().next()?;
    syn.quotes.contains(c).then_some((1, c, false, false, 0))
}

/// Where a string that is open at `from` ends on this line, and whether it closed.
fn string_end(line: &str, from: usize, quote: char, triple: bool, raw: bool, hashes: u8) -> (usize, bool) {
    let close: String = if triple { std::iter::repeat_n(quote, 3).collect() } else { std::iter::once(quote).chain(std::iter::repeat_n('#', hashes as usize)).collect() };
    let mut i = from;
    while i < line.len() {
        let rest = &line[i..];
        if !raw && rest.starts_with('\\') {
            i += 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
            continue;
        }
        if rest.starts_with(&close) {
            return (i + close.len(), true);
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    (line.len(), false)
}

/// The length of a Rust char literal at the start of `s`, if there is one.
fn rust_char(s: &str) -> Option<usize> {
    let body = s.strip_prefix('\'')?;
    let len = if let Some(esc) = body.strip_prefix('\\') {
        if let Some(u) = esc.strip_prefix("u{") {
            3 + u.find('}')? + 1
        } else {
            1 + esc.chars().next()?.len_utf8()
        }
    } else {
        body.chars().next().filter(|c| *c != '\'')?.len_utf8()
    };
    body[len..].starts_with('\'').then_some(1 + len + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tokens of `line` as (text, token) pairs, for reading the tests.
    fn toks(lang: &str, lines: &[&str]) -> Vec<Vec<(String, Token)>> {
        lex_block(Lang::new(lang), lines)
            .into_iter()
            .zip(lines)
            .map(|(ts, line)| ts.into_iter().map(|(r, t)| (line[r].to_string(), t)).collect())
            .collect()
    }

    fn of(lang: &str, line: &str, token: Token) -> Vec<String> {
        toks(lang, &[line])[0].iter().filter(|(_, t)| *t == token).map(|(s, _)| s.clone()).collect()
    }

    #[std::prelude::v1::test]
    fn rust_tokens() {
        let line = "    assert_eq!(finish(Pace::new(4, 55)), secs(2950)); // 49:10, with 50 s to spare";
        assert_eq!(of("rust", line, Token::Macro), ["assert_eq!"]);
        assert_eq!(of("rust", line, Token::Type), ["Pace"]);
        assert_eq!(of("rust", line, Token::Number), ["4", "55", "2950"]);
        assert_eq!(of("rust", line, Token::Comment), ["// 49:10, with 50 s to spare"]);
        assert!(of("rust", line, Token::Punct).iter().any(|p| p == ":"));
        let line = "fn finish(pace: Pace) -> Duration {";
        assert_eq!(of("rust", line, Token::Keyword), ["fn"]);
        assert_eq!(of("rust", line, Token::Type), ["Pace", "Duration"]);
        assert_eq!(of("rust", "let per_km = pace.min as u64 * 60;", Token::Keyword), ["let", "as"]);
        assert_eq!(of("rust", "let per_km = pace.min as u64 * 60;", Token::Type), ["u64"]);
        assert_eq!(of("rust", "#[derive(Debug, Clone)]", Token::Macro), ["#[derive(Debug, Clone)]"]);
        // chars, lifetimes, byte and raw strings, ranges
        assert_eq!(of("rust", "let c = '\\n'; fn f<'a>(x: &'a str) {}", Token::String), ["'\\n'"]);
        assert_eq!(of("rust", "fn f<'a>(x: &'a str) {}", Token::Type), ["'a", "'a", "str"]);
        assert_eq!(of("rust", r##"let s = r#"a "quoted" b"#; let t = b"x";"##, Token::String), [r##"r#"a "quoted" b"#"##, r#"b"x""#]);
        assert_eq!(of("rust", "for i in 1..10 {}", Token::Number), ["1", "10"]);
        assert_eq!(of("rust", "let x = 0x1f_u8 + 1.5e3 + 2.0f32;", Token::Number), ["0x1f_u8", "1.5e3", "2.0f32"]);
        assert!(of("rust", "if a != b {}", Token::Macro).is_empty(), "!= is not a macro");
    }

    #[std::prelude::v1::test]
    fn comments_and_strings_carry_across_lines() {
        let t = toks("rust", &["let s = \"one", "two\"; /* a", "still */ x", "// done"]);
        assert_eq!(t[0].last().unwrap(), &("\"one".to_string(), Token::String));
        assert_eq!(t[1][0], ("two\"".to_string(), Token::String));
        assert_eq!(t[1].last().unwrap(), &("/* a".to_string(), Token::Comment));
        assert_eq!(t[2][0], ("still */".to_string(), Token::Comment));
        assert_eq!(t[3], [("// done".to_string(), Token::Comment)]);
        // Python's triple quotes, with a prefix
        let t = toks("python", &["doc = f\"\"\"Pace", "per km\"\"\" # note", "x = 'a'"]);
        assert_eq!(t[0].last().unwrap(), &("f\"\"\"Pace".to_string(), Token::String));
        assert_eq!(t[1][0], ("per km\"\"\"".to_string(), Token::String));
        assert_eq!(t[1].last().unwrap(), &("# note".to_string(), Token::Comment));
        assert_eq!(t[2].iter().filter(|(_, k)| *k == Token::String).count(), 1);
        // a JavaScript template runs on; a plain string does not
        let t = toks("ts", &["const a = `one", "two` + 'x", "y = 1"]);
        assert_eq!(t[1][0], ("two`".to_string(), Token::String));
        assert_eq!(t[2].iter().filter(|(_, k)| *k == Token::String).count(), 0, "a quote string stops at the line end");
        // escapes do not close a string
        assert_eq!(of("js", r#"x = "a \"b\" c" + 1"#, Token::String), [r#""a \"b\" c""#]);
    }

    #[std::prelude::v1::test]
    fn other_languages() {
        assert_eq!(of("python", "@cache", Token::Macro), ["@cache"]);
        assert_eq!(of("python", "def finish(pace: Pace) -> int:", Token::Keyword), ["def"]);
        assert_eq!(of("python", "def finish(pace: Pace) -> int:", Token::Type), ["Pace", "int"]);
        assert_eq!(of("go", "func main() { var x int = 5 }", Token::Keyword), ["func", "var"]);
        assert_eq!(of("sql", "SELECT name FROM runs WHERE km > 10 -- long", Token::Keyword), ["SELECT", "FROM", "WHERE"]);
        assert_eq!(of("sql", "SELECT name FROM runs WHERE km > 10 -- long", Token::Comment), ["-- long"]);
        assert_eq!(of("bash", "echo \"$HOME\" ${PATH} $1 # comment", Token::Macro), ["${PATH}", "$1"]);
        assert_eq!(of("bash", "echo a#b # comment", Token::Comment), ["# comment"]);
        assert_eq!(of("json", r#"{"pace": 5.5, "done": true}"#, Token::Keyword), ["true"]);
        assert_eq!(of("json", r#"{"pace": 5.5, "done": true}"#, Token::Number), ["5.5"]);
        // languages it does not know are plain, and so is no language at all
        assert!(toks("brainfuck", &["+++."])[0].is_empty());
        assert!(toks("", &["fn main() {}"])[0].is_empty());
        assert!(knows(Lang::new("RS")) && !knows(Lang::new("")));
        // never a panic on odd text: unclosed things, multibyte characters
        for line in ["'", "r#\"", "\"\\", "/*", "#[", "${", "é'à'", "0x", ".5", "@", "$", "'\\u{2019"] {
            for lang in ["rust", "python", "js", "go", "sql", "bash", "json", "css", "c"] {
                let t = lex_block(Lang::new(lang), &[line, line]);
                for ts in &t {
                    for (r, _) in ts {
                        assert!(line.is_char_boundary(r.start) && line.is_char_boundary(r.end) && r.start < r.end, "{lang} {line:?} {r:?}");
                    }
                }
            }
        }
    }

    /// WCAG contrast ratio of two opaque colours.
    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let lum = |h: Hsla| {
            let c = h.to_rgb();
            let f = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[std::prelude::v1::test]
    fn every_colour_reads_on_the_code_ground() {
        for p in [Palette::light(), Palette::dark()] {
            let light = if p.dark { "lamplight" } else { "linen" };
            for t in Token::ALL {
                let ratio = contrast(color(t, &p), p.code);
                assert!(ratio >= 4.5, "{t:?} on the code ground in {light}: {ratio:.2}:1");
            }
            assert!(contrast(p.ink, p.code) >= 4.5, "plain code in {light}");
            // never Inkling's madder, or rose beside it
            for t in Token::ALL {
                for ink in [InkColor::Ink, InkColor::Red] {
                    let (l, d) = thread_hex(ink);
                    assert_ne!(color(t, &p), hex(if p.dark { d } else { l }), "{t:?} is {ink:?}");
                }
            }
        }
        // the ground is a step off the page: darker in linen, lighter in lamplight
        assert!(Palette::light().code.l < Palette::light().paper.l);
        assert!(Palette::dark().code.l > Palette::dark().paper.l);
    }
}
