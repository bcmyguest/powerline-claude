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

/// Remaining rate-limit budget (percent left in the tightest window) below
/// which the usage segment turns orange, then red.
pub const USAGE_WARN_REMAINING: f64 = 20.0;
pub const USAGE_ALERT_REMAINING: f64 = 5.0;

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
    /// cost. The context and usage rows are their calm states — past the
    /// token / remaining-budget thresholds they swap to the warn/alert
    /// families (see `Module::colors`).
    fg: Family,
    bg: Family,
    /// Keep order when the bar overflows the terminal: lower-priority
    /// segments are dropped first until the bar fits (see `lib.rs::run`).
    priority: u8,
}

/// The module registry. Table order is the default bar order; adding a
/// module means adding a row here plus a formatting arm in `segment_texts`.
const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        module: Module::Logo,
        name: "logo",
        fg: Family::Claude,
        bg: Family::Claude,
        priority: 1,
    },
    ModuleSpec {
        module: Module::Dir,
        name: "dir",
        fg: Family::Directory,
        bg: Family::Directory,
        priority: 8,
    },
    ModuleSpec {
        module: Module::Git,
        name: "git",
        fg: Family::Git,
        bg: Family::Git,
        priority: 7,
    },
    ModuleSpec {
        module: Module::Model,
        name: "model",
        fg: Family::Model,
        bg: Family::Model,
        priority: 6,
    },
    ModuleSpec {
        module: Module::Context,
        name: "context",
        fg: Family::Context,
        bg: Family::Context,
        priority: 9,
    },
    ModuleSpec {
        module: Module::Cost,
        name: "cost",
        fg: Family::Cost,
        bg: Family::Cost,
        priority: 4,
    },
    ModuleSpec {
        module: Module::Usage,
        name: "usage",
        fg: Family::Cost,
        bg: Family::Cost,
        priority: 5,
    },
    ModuleSpec {
        module: Module::Stats,
        name: "stats",
        fg: Family::Cost,
        bg: Family::Context,
        priority: 2,
    },
    ModuleSpec {
        module: Module::Effort,
        name: "effort",
        fg: Family::Model,
        bg: Family::Model,
        priority: 3,
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
    /// Context, usage, and git are the state-dependent modules: past the
    /// token thresholds (context), below the remaining-budget thresholds
    /// (usage), or during an in-progress repo operation (git, read from the
    /// git dir like the branch is) they paint with the warn/alert families
    /// instead of their registry rows, which is why the payload comes along.
    pub fn colors(self, theme: &Theme, payload: &Payload) -> SegmentColors {
        let spec = self.spec();
        let (fg, bg) = match self {
            Module::Context => {
                let family = context_family(payload.current_tokens());
                (family, family)
            }
            Module::Usage => {
                let family = usage_family(payload.five_hour_used(), payload.seven_day_used());
                (family, family)
            }
            Module::Git
                if payload
                    .dir()
                    .is_some_and(|dir| git_state(Path::new(dir)).is_some()) =>
            {
                (Family::ContextWarn, Family::ContextWarn)
            }
            _ => (spec.fg, spec.bg),
        };
        SegmentColors {
            fg: theme.family(fg).fg,
            bg: theme.family(bg).bg,
        }
    }

    /// Keep order when the bar overflows: the lowest-priority segment is
    /// dropped first.
    pub fn priority(self) -> u8 {
        self.spec().priority
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

pub fn format_tokens(tokens: Option<u64>, columns: usize) -> String {
    match tokens {
        Some(count) if count > 0 && columns < NARROW_COLUMNS => {
            format!("{} tok", compact_number(count))
        }
        Some(count) if count > 0 => format!("{} tok", group_thousands(count)),
        _ => "~~ tok".to_string(),
    }
}

/// `15.5k` / `1.2M`-style count for narrow terminals, one decimal with a
/// trailing `.0` trimmed.
fn compact_number(value: u64) -> String {
    let (scaled, unit) = if value >= 1_000_000 {
        (value as f64 / 1_000_000.0, "M")
    } else if value >= 1_000 {
        (value as f64 / 1_000.0, "k")
    } else {
        return value.to_string();
    };
    let text = format!("{scaled:.1}");
    let trimmed = text.strip_suffix(".0").unwrap_or(&text);
    format!("{trimmed}{unit}")
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

/// One subscription rate-limit window as the payload reports it.
#[derive(Debug, Default, Clone, Copy)]
pub struct UsageWindow {
    pub used_percentage: Option<f64>,
    /// Unix timestamp (seconds) at which the window resets.
    pub resets_at: Option<i64>,
}

/// Remaining subscription rate-limit budget, from the used percentages the
/// payload reports per window, plus the time until each window resets when
/// the clock is known: `5h 77% (2h) · 7d 59% (5d)`. Windows absent from the
/// payload are dropped; no windows at all means no segment.
pub fn format_usage(
    five_hour: UsageWindow,
    seven_day: UsageWindow,
    now: Option<i64>,
) -> Option<String> {
    let parts: Vec<String> = [("5h", five_hour), ("7d", seven_day)]
        .into_iter()
        .filter_map(|(label, window)| usage_part(label, window, now))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

fn usage_part(label: &str, window: UsageWindow, now: Option<i64>) -> Option<String> {
    let remaining = (100.0 - window.used_percentage?).clamp(0.0, 100.0).round();
    let mut part = format!("{label} {remaining:.0}%");
    if let (Some(resets_at), Some(now)) = (window.resets_at, now)
        && resets_at > now
    {
        let _ = write!(part, " ({})", format_reset(resets_at - now));
    }
    Some(part)
}

/// Coarse time-until-reset: whole days, else whole hours, else minutes.
fn format_reset(seconds: i64) -> String {
    let minutes = seconds / 60;
    if minutes >= 24 * 60 {
        format!("{}d", minutes / (24 * 60))
    } else if minutes >= 60 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}m", minutes.max(1))
    }
}

/// Which family paints the usage segment: its registry row (cost) normally,
/// warn/alert when the tightest reported window is nearly spent.
pub fn usage_family(five_hour_used: Option<f64>, seven_day_used: Option<f64>) -> Family {
    let least_remaining = [five_hour_used, seven_day_used]
        .into_iter()
        .flatten()
        .map(|used| (100.0 - used).clamp(0.0, 100.0))
        .fold(f64::INFINITY, f64::min);
    if least_remaining < USAGE_ALERT_REMAINING {
        Family::ContextAlert
    } else if least_remaining < USAGE_WARN_REMAINING {
        Family::ContextWarn
    } else {
        Family::Cost
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
    branch_from_head(&git_head_dir(dir)?.join("HEAD"))
}

/// In-progress repository operation, detected from the marker files git
/// leaves next to `HEAD` (in a linked worktree that's the worktree's own
/// gitdir, where these markers live). A handful of stat calls, no
/// subprocess.
pub fn git_state(dir: &Path) -> Option<&'static str> {
    let gitdir = git_head_dir(dir)?;
    if gitdir.join("rebase-merge").is_dir() || gitdir.join("rebase-apply").is_dir() {
        Some("rebasing")
    } else if gitdir.join("MERGE_HEAD").is_file() {
        Some("merging")
    } else if gitdir.join("CHERRY_PICK_HEAD").is_file() {
        Some("cherry-picking")
    } else if gitdir.join("REVERT_HEAD").is_file() {
        Some("reverting")
    } else if gitdir.join("BISECT_LOG").is_file() {
        Some("bisecting")
    } else {
        None
    }
}

/// The directory holding this worktree's `HEAD`: `.git` itself, or the
/// gitdir a worktree's `.git` file points at, walking up from `dir`.
fn git_head_dir(dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = Some(dir);
    while let Some(candidate) = current {
        let dotgit = candidate.join(".git");
        if dotgit.is_dir() {
            return Some(dotgit);
        }
        if dotgit.is_file() {
            let contents = std::fs::read_to_string(&dotgit).ok()?;
            let gitdir = contents.strip_prefix("gitdir:")?.trim();
            return Some(candidate.join(gitdir));
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
    now: Option<i64>,
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
                Module::Context => Some(format_tokens(payload.current_tokens(), columns)),
                Module::Cost => payload.total_cost_usd().map(format_cost),
                Module::Usage => format_usage(
                    UsageWindow {
                        used_percentage: payload.five_hour_used(),
                        resets_at: payload.five_hour_resets_at(),
                    },
                    UsageWindow {
                        used_percentage: payload.seven_day_used(),
                        resets_at: payload.seven_day_resets_at(),
                    },
                    now,
                ),
                Module::Stats => payload.total_duration_ms().map(format_duration),
                Module::Effort => payload.effort_level().map(str::to_string),
            };
            text.map(|text| (*module, text))
        })
        .collect()
}

/// Branch, any in-progress repo operation, then the session's line churn:
/// ` main (merging) +156 -23`. Lines come from the payload (what Claude
/// changed), not from a git diff.
fn git_segment(payload: &Payload, mode: Mode) -> Option<String> {
    let dir = Path::new(payload.dir()?);
    let branch = git_branch(dir)?;
    let mut text = format!("{} {branch}", branch_icon(mode));
    if let Some(state) = git_state(dir) {
        let _ = write!(text, " ({state})");
    }
    if let (Some(added), Some(removed)) = (payload.lines_added(), payload.lines_removed()) {
        let _ = write!(text, " +{added} -{removed}");
    }
    Some(text)
}
