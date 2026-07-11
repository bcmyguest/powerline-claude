//! Segment content: what each module renders, given the payload.
//!
//! `MODULES` is the registry — the one table that defines every module's CLI
//! name, default position, and palette families. Everything here is pure
//! string building except `git_branch`, which reads `.git/HEAD` from disk
//! (never a subprocess — the statusline runs on every render and must stay
//! cheap).

use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;

use crate::payload::Payload;
use crate::render::Mode;
use crate::theme::{Family, SegmentColors, Theme};

const LOGO: &str = "\u{f4f5}";
const HAIKU_ICON: char = '\u{ee0d}';
const SONNET_ICON: char = '\u{f06a9}';
const OPUS_ICON: char = '\u{f16a6}';
const BRANCH_ICON: char = '\u{e0a0}';

// Compatible mode promises a bar that renders without a patched font, so the
// nerd-font glyphs above get plain-Unicode stand-ins (and the model segment
// simply drops its icon).
const LOGO_COMPAT: &str = "\u{2733}"; // ✳
const BRANCH_ICON_COMPAT: char = '\u{2387}'; // ⎇

fn logo(mode: Mode) -> &'static str {
    match mode {
        Mode::Compatible => LOGO_COMPAT,
        _ => LOGO,
    }
}

fn branch_icon(mode: Mode) -> char {
    match mode {
        Mode::Compatible => BRANCH_ICON_COMPAT,
        _ => BRANCH_ICON,
    }
}

/// Terminals narrower than this get a more aggressive dir truncation.
const NARROW_COLUMNS: usize = 80;

/// Context tokens at which the context segment turns orange, then red.
pub const CONTEXT_WARN_TOKENS: u64 = 80_000;
pub const CONTEXT_ALERT_TOKENS: u64 = 125_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    Logo,
    Dir,
    Git,
    Model,
    Context,
    Cost,
    Usage,
    Stats,
    Effort,
}

/// One registry row: everything the rest of the program needs to know about
/// a module besides how to format its text.
struct ModuleSpec {
    module: Module,
    /// CLI name, as it appears in `--modules`.
    name: &'static str,
    /// Which palette family paints the foreground / background. Most modules
    /// use one family for both; stats, effort, and usage have no family of
    /// their own and borrow: stats paints the cost fg on the context bg
    /// (keeping the alternating-bg rhythm), effort paints as model, usage as
    /// cost. Context's row is its calm state — past the token thresholds it
    /// swaps to the warn/alert families (see `Module::colors`).
    fg: Family,
    bg: Family,
}

/// The module registry. Table order is the default bar order; adding a
/// module means adding a row here plus a formatting arm in `segment_texts`.
const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        module: Module::Logo,
        name: "logo",
        fg: Family::Claude,
        bg: Family::Claude,
    },
    ModuleSpec {
        module: Module::Dir,
        name: "dir",
        fg: Family::Directory,
        bg: Family::Directory,
    },
    ModuleSpec {
        module: Module::Git,
        name: "git",
        fg: Family::Git,
        bg: Family::Git,
    },
    ModuleSpec {
        module: Module::Model,
        name: "model",
        fg: Family::Model,
        bg: Family::Model,
    },
    ModuleSpec {
        module: Module::Context,
        name: "context",
        fg: Family::Context,
        bg: Family::Context,
    },
    ModuleSpec {
        module: Module::Cost,
        name: "cost",
        fg: Family::Cost,
        bg: Family::Cost,
    },
    ModuleSpec {
        module: Module::Usage,
        name: "usage",
        fg: Family::Cost,
        bg: Family::Cost,
    },
    ModuleSpec {
        module: Module::Stats,
        name: "stats",
        fg: Family::Cost,
        bg: Family::Context,
    },
    ModuleSpec {
        module: Module::Effort,
        name: "effort",
        fg: Family::Model,
        bg: Family::Model,
    },
];

impl Module {
    fn spec(self) -> &'static ModuleSpec {
        MODULES
            .iter()
            .find(|spec| spec.module == self)
            .expect("every Module variant has a registry row")
    }

    /// CLI name, as accepted by `--modules`.
    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// All module names, in default bar order.
    pub fn names() -> impl Iterator<Item = &'static str> {
        MODULES.iter().map(|spec| spec.name)
    }

    /// This module's colors under `theme`, per its registry families.
    /// Context is the one payload-dependent module: past the token
    /// thresholds it paints with the warn/alert families instead of its
    /// registry row, which is why the current token count comes along.
    pub fn colors(self, theme: &Theme, context_tokens: Option<u64>) -> SegmentColors {
        let spec = self.spec();
        let (fg, bg) = match self {
            Module::Context => {
                let family = context_family(context_tokens);
                (family, family)
            }
            _ => (spec.fg, spec.bg),
        };
        SegmentColors {
            fg: theme.family(fg).fg,
            bg: theme.family(bg).bg,
        }
    }

    pub fn default_order() -> Vec<Module> {
        MODULES.iter().map(|spec| spec.module).collect()
    }

    /// The default `--modules` value: every module, in registry order.
    pub fn default_list() -> String {
        Self::names().collect::<Vec<_>>().join(",")
    }
}

impl FromStr for Module {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MODULES
            .iter()
            .find(|spec| spec.name == s)
            .map(|spec| spec.module)
            .ok_or_else(|| {
                format!(
                    "unknown module '{s}', available: {}",
                    Module::default_list().replace(',', ", ")
                )
            })
    }
}

pub fn parse_modules(list: &str) -> Result<Vec<Module>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(Module::from_str)
        .collect()
}

pub fn format_model(display_name: &str, mode: Mode) -> String {
    let name = display_name.to_lowercase();
    if mode == Mode::Compatible {
        return name;
    }
    let icon = if name.contains("haiku") {
        HAIKU_ICON
    } else if name.contains("opus") {
        OPUS_ICON
    } else {
        SONNET_ICON
    };
    format!("{icon} {name}")
}

pub fn format_tokens(tokens: Option<u64>) -> String {
    match tokens {
        Some(count) if count > 0 => format!("{} tok", group_thousands(count)),
        _ => "~~ tok".to_string(),
    }
}

fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn format_cost(usd: f64) -> String {
    format!("${usd:.2}")
}

/// Which family paints the context segment at this token count: normal
/// below the warn threshold, orange from 80k, red from 125k.
pub fn context_family(tokens: Option<u64>) -> Family {
    match tokens {
        Some(count) if count >= CONTEXT_ALERT_TOKENS => Family::ContextAlert,
        Some(count) if count >= CONTEXT_WARN_TOKENS => Family::ContextWarn,
        _ => Family::Context,
    }
}

/// Remaining subscription rate-limit budget, from the used percentages the
/// payload reports per window: `5h 77% · 7d 59%`. Windows absent from the
/// payload are dropped; no windows at all means no segment.
pub fn format_usage(five_hour_used: Option<f64>, seven_day_used: Option<f64>) -> Option<String> {
    let remaining =
        |used: Option<f64>| used.map(|percent| (100.0 - percent).clamp(0.0, 100.0).round());
    let parts: Vec<String> = [
        ("5h", remaining(five_hour_used)),
        ("7d", remaining(seven_day_used)),
    ]
    .into_iter()
    .filter_map(|(label, left)| left.map(|left| format!("{label} {left:.0}%")))
    .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

pub fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{total_seconds}s")
    }
}

/// Shorten a path for display: home becomes `~`, and anything deeper keeps
/// only the last two components (one on narrow terminals), starship-style.
pub fn truncate_dir(path: &str, home: &str, columns: usize) -> String {
    let display = match path.strip_prefix(home) {
        Some("") => return "~".to_string(),
        Some(rest) => format!("~{rest}"),
        None => path.to_string(),
    };
    let keep = if columns < NARROW_COLUMNS { 1 } else { 2 };
    let components: Vec<&str> = display.split('/').filter(|c| !c.is_empty()).collect();
    if components.len() <= keep || (display.starts_with('~') && components.len() <= keep + 1) {
        return display;
    }
    components[components.len() - keep..].join("/")
}

/// Current branch, read straight from `.git/HEAD`, walking up from `dir` and
/// following worktree gitfiles. Detached HEAD yields the short hash.
pub fn git_branch(dir: &Path) -> Option<String> {
    let mut current = Some(dir);
    while let Some(candidate) = current {
        let dotgit = candidate.join(".git");
        if dotgit.is_dir() {
            return branch_from_head(&dotgit.join("HEAD"));
        }
        if dotgit.is_file() {
            let contents = std::fs::read_to_string(&dotgit).ok()?;
            let gitdir = contents.strip_prefix("gitdir:")?.trim();
            return branch_from_head(&candidate.join(gitdir).join("HEAD"));
        }
        current = candidate.parent();
    }
    None
}

fn branch_from_head(head_path: &Path) -> Option<String> {
    let head = std::fs::read_to_string(head_path).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        None => Some(head.chars().take(8).collect()),
    }
}

/// Build the text for every requested module, skipping modules whose data is
/// absent from the payload. Returns `(module, text)` pairs ready for theming.
/// `home` comes from the caller (see `Env`) so nothing here reads the
/// process environment.
pub fn segment_texts(
    payload: &Payload,
    modules: &[Module],
    columns: usize,
    home: &str,
    mode: Mode,
) -> Vec<(Module, String)> {
    modules
        .iter()
        .filter_map(|module| {
            let text = match module {
                Module::Logo => Some(logo(mode).to_string()),
                Module::Dir => payload.dir().map(|dir| truncate_dir(dir, home, columns)),
                Module::Git => git_segment(payload, mode),
                Module::Model => payload
                    .model_display_name()
                    .map(|name| format_model(name, mode)),
                Module::Context => Some(format_tokens(payload.current_tokens())),
                Module::Cost => payload.total_cost_usd().map(format_cost),
                Module::Usage => format_usage(payload.five_hour_used(), payload.seven_day_used()),
                Module::Stats => payload.total_duration_ms().map(format_duration),
                Module::Effort => payload.effort_level().map(str::to_string),
            };
            text.map(|text| (*module, text))
        })
        .collect()
}

/// Branch plus the session's line churn: ` main +156 -23`. Lines come from
/// the payload (what Claude changed), not from a git diff.
fn git_segment(payload: &Payload, mode: Mode) -> Option<String> {
    let branch = git_branch(Path::new(payload.dir()?))?;
    let mut text = format!("{} {branch}", branch_icon(mode));
    if let (Some(added), Some(removed)) = (payload.lines_added(), payload.lines_removed()) {
        let _ = write!(text, " +{added} -{removed}");
    }
    Some(text)
}
