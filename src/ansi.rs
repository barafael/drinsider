//! Minimal ANSI SGR parsing for lurk's colored output.
// The browser build has no tracer, so nothing produces colored lines there.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::ops::Range;

/// Terminal color of a span, mapped onto the theme later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tone {
    /// Default text.
    #[default]
    Plain,
    /// Bold or white.
    Strong,
    /// Red.
    Red,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Blue.
    Blue,
    /// Magenta.
    Magenta,
    /// Cyan.
    Cyan,
}

/// A line of text with colored byte ranges.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    /// Text without escape sequences.
    pub text: String,
    /// Colored ranges of `text`, in order, non-overlapping.
    pub spans: Vec<(Range<usize>, Tone)>,
}

/// Parses SGR color sequences out of `raw`; other escapes are dropped.
pub fn parse(raw: &str) -> Line {
    let mut line = Line::default();
    let (mut color, mut bold) = (None::<Tone>, false);
    let mut chars = raw.chars().peekable();
    let mut span_start = 0;
    let mut current = Tone::Plain;
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            line.text.push(c);
            continue;
        }
        if chars.peek() != Some(&'[') {
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut terminator = None;
        for c in chars.by_ref() {
            if c.is_ascii_digit() || c == ';' {
                params.push(c);
            } else {
                terminator = Some(c);
                break;
            }
        }
        if terminator != Some('m') {
            continue;
        }
        let mut codes = params.split(';').map(|p| p.parse::<u16>().unwrap_or(0));
        while let Some(code) = codes.next() {
            match code {
                0 => (color, bold) = (None, false),
                1 => bold = true,
                22 => bold = false,
                31 | 91 => color = Some(Tone::Red),
                32 | 92 => color = Some(Tone::Green),
                33 | 93 => color = Some(Tone::Yellow),
                34 | 94 => color = Some(Tone::Blue),
                35 | 95 => color = Some(Tone::Magenta),
                36 | 96 => color = Some(Tone::Cyan),
                37 | 97 => color = Some(Tone::Strong),
                30 | 39 | 90 => color = None,
                // 256-color and truecolor: skip their arguments.
                38 | 48 => match codes.next() {
                    Some(5) => {
                        codes.next();
                    }
                    Some(2) => {
                        codes.next();
                        codes.next();
                        codes.next();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let tone = color.unwrap_or(if bold { Tone::Strong } else { Tone::Plain });
        if tone != current {
            if line.text.len() > span_start && current != Tone::Plain {
                line.spans.push((span_start..line.text.len(), current));
            }
            span_start = line.text.len();
            current = tone;
        }
    }
    if line.text.len() > span_start && current != Tone::Plain {
        line.spans.push((span_start..line.text.len(), current));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_and_reset() {
        let line = parse("\x1b[36m[42]\x1b[0m \x1b[1mread\x1b[0m(3) = \x1b[32;1m0\x1b[0m");
        assert_eq!(line.text, "[42] read(3) = 0");
        assert_eq!(
            line.spans,
            vec![(0..4, Tone::Cyan), (5..9, Tone::Strong), (15..16, Tone::Green)]
        );
    }

    #[test]
    fn plain_text_and_garbage() {
        assert_eq!(parse("plain").spans, vec![]);
        assert_eq!(parse("a\x1b[38;5;196mb\x1b[0mc").text, "abc");
        assert_eq!(parse("\x1b[2Kx").text, "x");
    }
}
