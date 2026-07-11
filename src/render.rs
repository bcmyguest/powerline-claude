//! Turns a list of colored segments into one ANSI powerline bar.

use std::fmt::Write;
use std::str::FromStr;

use crate::theme::{Rgb, SegmentColors};

const RESET: &str = "\x1b[0m";

/// Separator glyphs for one rendering mode. `hard` bridges two different
/// backgrounds, `thin` divides two segments sharing a background. The
/// `*_right` variants are the mirrored glyphs for a right-aligned bar.
struct Separators {
    hard: &'static str,
    thin: &'static str,
    fade: &'static str,
    hard_right: &'static str,
    thin_right: &'static str,
    fade_right: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Nerd-font powerline glyphs (private-use area).
    Patched,
    /// Standard Unicode stand-ins for terminals without patched fonts.
    Compatible,
    /// No separators at all.
    Flat,
}

impl Mode {
    /// CLI names for `--mode`, in documentation order.
    pub const NAMES: [&'static str; 3] = ["patched", "compatible", "flat"];

    fn separators(self) -> Option<Separators> {
        match self {
            Mode::Patched => Some(Separators {
                hard: "\u{e0b0}",
                thin: "\u{e0b1}",
                fade: "░▒▓",
                hard_right: "\u{e0b2}",
                thin_right: "\u{e0b3}",
                fade_right: "▓▒░",
            }),
            Mode::Compatible => Some(Separators {
                hard: "▶",
                thin: "❯",
                fade: "░▒▓",
                hard_right: "◀",
                thin_right: "❮",
                fade_right: "▓▒░",
            }),
            Mode::Flat => None,
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "patched" => Ok(Mode::Patched),
            "compatible" => Ok(Mode::Compatible),
            "flat" => Ok(Mode::Flat),
            other => Err(format!(
                "unknown mode '{other}', available: {}",
                Mode::NAMES.join(", ")
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub colors: SegmentColors,
}

fn fg(color: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
}

fn bg(color: Rgb) -> String {
    format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b)
}

pub fn render(segments: &[Segment], mode: Mode) -> String {
    let Some(first) = segments.first() else {
        return String::new();
    };
    let separators = mode.separators();
    let mut out = String::new();

    if let Some(sep) = &separators {
        let _ = write!(out, "{}{}", fg(first.colors.bg), sep.fade);
    }

    for (i, segment) in segments.iter().enumerate() {
        if let (Some(sep), Some(previous)) = (&separators, i.checked_sub(1).map(|p| &segments[p])) {
            if previous.colors.bg == segment.colors.bg {
                let _ = write!(
                    out,
                    "{}{}{}",
                    bg(segment.colors.bg),
                    fg(segment.colors.fg),
                    sep.thin
                );
            } else {
                let _ = write!(
                    out,
                    "{}{}{}",
                    bg(segment.colors.bg),
                    fg(previous.colors.bg),
                    sep.hard
                );
            }
        }
        let _ = write!(
            out,
            "{}{} {} ",
            bg(segment.colors.bg),
            fg(segment.colors.fg),
            segment.text
        );
    }

    let last = segments.last().unwrap_or(first);
    match &separators {
        Some(sep) => {
            let _ = write!(out, "{RESET}{}{}{RESET}", fg(last.colors.bg), sep.hard);
        }
        None => out.push_str(RESET),
    }
    out
}

/// Mirror of `render` for a right-aligned bar: opens with a left-pointing
/// arrow on the default background, closes with the fade at the terminal's
/// right edge.
pub fn render_right(segments: &[Segment], mode: Mode) -> String {
    let Some(first) = segments.first() else {
        return String::new();
    };
    let separators = mode.separators();
    let mut out = String::new();

    if let Some(sep) = &separators {
        let _ = write!(out, "{}{}", fg(first.colors.bg), sep.hard_right);
    }

    for (i, segment) in segments.iter().enumerate() {
        if let (Some(sep), Some(previous)) = (&separators, i.checked_sub(1).map(|p| &segments[p])) {
            if previous.colors.bg == segment.colors.bg {
                let _ = write!(
                    out,
                    "{}{}{}",
                    bg(segment.colors.bg),
                    fg(segment.colors.fg),
                    sep.thin_right
                );
            } else {
                // mirrored: the triangle is filled with the bg of the segment
                // to its right, over the bg of the segment to its left
                let _ = write!(
                    out,
                    "{}{}{}",
                    bg(previous.colors.bg),
                    fg(segment.colors.bg),
                    sep.hard_right
                );
            }
        }
        let _ = write!(
            out,
            "{}{} {} ",
            bg(segment.colors.bg),
            fg(segment.colors.fg),
            segment.text
        );
    }

    let last = segments.last().unwrap_or(first);
    match &separators {
        Some(sep) => {
            let _ = write!(
                out,
                "{RESET}{}{}{RESET}",
                fg(last.colors.bg),
                sep.fade_right
            );
        }
        None => out.push_str(RESET),
    }
    out
}

/// Columns the bar occupies on screen: characters outside SGR escape
/// sequences (`ESC [ ... m`), each counted as one cell — true for every
/// glyph this renderer emits.
pub fn visible_width(rendered: &str) -> usize {
    let mut count = 0;
    let mut chars = rendered.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            count += 1;
        }
    }
    count
}
