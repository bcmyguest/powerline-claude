//! powerline-claude: a powerline-style status line for Claude Code.
//!
//! `run` is the whole program: statusline JSON and ambient state in, every
//! byte the program prints out. The binary in `main.rs` only wires stdin,
//! stdout, and `/dev/tty` around it.

pub mod cli;
pub mod payload;
mod progress;
pub mod render;
pub mod segments;
pub mod theme;

use cli::Cli;
use payload::Payload;
use render::Segment;

const DEFAULT_WIDTH: usize = 200;

/// Ambient process state the bar depends on, passed in explicitly so `run`
/// stays pure and fixture-driven tests are deterministic on any machine.
#[derive(Debug, Clone)]
pub struct Env {
    /// `$HOME`, for tilde-shortening the dir segment.
    pub home: String,
    /// Raw `$COLUMNS` value (set by Claude Code v2.1.153+).
    pub columns: Option<String>,
}

impl Env {
    pub fn from_process() -> Self {
        Self {
            home: std::env::var("HOME").unwrap_or_default(),
            columns: std::env::var("COLUMNS").ok(),
        }
    }
}

/// Everything the program prints, decided in one place.
#[derive(Debug)]
pub struct Output {
    /// The ANSI powerline bar, for stdout.
    pub bar: String,
    /// OSC 9;4 progress sequence for the controlling terminal, present when
    /// the payload has context numbers and `--no-progress` is off.
    pub progress: Option<String>,
}

pub fn run(
    payload_json: &str,
    cli: &Cli,
    env: &Env,
    tty_width: impl FnOnce() -> Option<usize>,
) -> Result<Output, String> {
    let payload = Payload::from_json(payload_json).map_err(|e| format!("bad payload: {e}"))?;
    let theme = theme::Theme::by_name(&cli.theme)?;
    let modules = segments::parse_modules(&cli.modules)?;
    let modules_right = segments::parse_modules(&cli.modules_right)?;
    let mode: render::Mode = cli.mode.parse()?;
    let width = resolve_width(cli.width, env.columns.as_deref(), tty_width);

    let build = |modules: &[segments::Module]| -> Vec<Segment> {
        segments::segment_texts(&payload, modules, width, &env.home, mode)
            .into_iter()
            .map(|(module, text)| Segment {
                text,
                colors: module.colors(&theme, payload.current_tokens()),
            })
            .collect()
    };

    let left_bar = render::render(&build(&modules), mode);
    let right_segments = build(&modules_right);
    let bar = if right_segments.is_empty() {
        left_bar
    } else {
        // Pad the gap so the right bar ends at the terminal edge; when the
        // two sides don't fit, keep at least one space between them and let
        // the terminal do what it will.
        let right_bar = render::render_right(&right_segments, mode);
        let used = render::visible_width(&left_bar) + render::visible_width(&right_bar);
        let gap = width.saturating_sub(used).max(1);
        format!("{left_bar}{}{right_bar}", " ".repeat(gap))
    };

    let progress = if cli.no_progress {
        None
    } else {
        context_percent(&payload).map(progress::osc_sequence)
    };

    Ok(Output { bar, progress })
}

/// Terminal width precedence: explicit flag, then `$COLUMNS`, then the
/// parent-TTY probe (only consulted when the first two yield nothing — it
/// spawns subprocesses), then a wide default so nothing truncates needlessly.
pub fn resolve_width(
    flag: Option<usize>,
    columns_env: Option<&str>,
    tty_width: impl FnOnce() -> Option<usize>,
) -> usize {
    flag.or_else(|| columns_env.and_then(|value| value.parse().ok()))
        .or_else(tty_width)
        .unwrap_or(DEFAULT_WIDTH)
}

/// Context usage as a percentage of the window, when the payload has both
/// numbers. Drives the OSC progress bar.
fn context_percent(payload: &Payload) -> Option<u64> {
    let tokens = payload.current_tokens()?;
    let size = payload.context_window_size()?;
    if size == 0 || tokens == 0 {
        return None;
    }
    Some(tokens * 100 / size)
}
