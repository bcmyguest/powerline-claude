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
use crate::theme::{Family, SegmentColors, Theme};

const LOGO: &str = "\u{f4f5}";
const HAIKU_ICON: char = '\u{ee0d}';
const SONNET_ICON: char = '\u{f06a9}';
const OPUS_ICON: char = '\u{f16a6}';
const BRANCH_ICON: char = '\u{e0a0}';

/// Terminals narrower than this get a more aggressive dir truncation.
const NARROW_COLUMNS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    Logo,
    Dir,
    Git,
    Model,
    Context,
    Cost,
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
    /// use one family for both; stats and effort have no family of their own
    /// and borrow: stats paints the cost fg on the context bg (keeping the
    /// alternating-bg rhythm), effort paints as model.
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
    pub fn colors(self, theme: &Theme) -> SegmentColors {
        let spec = self.spec();
        SegmentColors {
            fg: theme.family(spec.fg).fg,
            bg: theme.family(spec.bg).bg,
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

pub fn format_model(display_name: &str) -> String {
    let name = display_name.to_lowercase();
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
pub fn segment_texts(
    payload: &Payload,
    modules: &[Module],
    columns: usize,
) -> Vec<(Module, String)> {
    let home = std::env::var("HOME").unwrap_or_default();
    modules
        .iter()
        .filter_map(|module| {
            let text = match module {
                Module::Logo => Some(LOGO.to_string()),
                Module::Dir => payload.dir().map(|dir| truncate_dir(dir, &home, columns)),
                Module::Git => git_segment(payload),
                Module::Model => payload.model_display_name().map(format_model),
                Module::Context => Some(format_tokens(payload.current_tokens())),
                Module::Cost => payload.total_cost_usd().map(format_cost),
                Module::Stats => payload.total_duration_ms().map(format_duration),
                Module::Effort => payload.effort_level().map(str::to_string),
            };
            text.map(|text| (*module, text))
        })
        .collect()
}

/// Branch plus the session's line churn: ` main +156 -23`. Lines come from
/// the payload (what Claude changed), not from a git diff.
fn git_segment(payload: &Payload) -> Option<String> {
    let branch = git_branch(Path::new(payload.dir()?))?;
    let mut text = format!("{BRANCH_ICON} {branch}");
    if let (Some(added), Some(removed)) = (payload.lines_added(), payload.lines_removed()) {
        let _ = write!(text, " +{added} -{removed}");
    }
    Some(text)
}
