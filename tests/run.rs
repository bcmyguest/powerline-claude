use powerline_claude::cli::Cli;
use powerline_claude::{Env, run};

use clap::Parser;

fn cli(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("powerline-claude").chain(args.iter().copied())).unwrap()
}

/// A fixed environment matching the fixtures, so goldens are the same on
/// every machine regardless of the real $HOME or $COLUMNS.
fn env() -> Env {
    Env {
        home: "/home/user".to_string(),
        columns: None,
    }
}

fn run_fixture(json: &str, args: &[&str]) -> Result<powerline_claude::Output, String> {
    run(json, &cli(args), &env(), || None)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                // CSI: consume to final byte (0x40-0x7e)
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&c) && c != '[' {
                            break;
                        }
                    }
                }
                // OSC: consume to BEL
                Some(']') => {
                    for c in chars.by_ref() {
                        if c == '\x07' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn cli_defaults_match_the_documented_interface() {
    let cli = cli(&[]);
    assert_eq!(
        cli.modules,
        "logo,dir,git,model,context,cost,usage,stats,effort"
    );
    assert_eq!(cli.modules_right, "");
    assert_eq!(cli.theme, "catppuccin-mocha");
    assert_eq!(cli.mode, "patched");
    assert!(!cli.no_progress);
    assert_eq!(cli.width, None);
}

#[test]
fn full_payload_renders_every_data_backed_segment() {
    // fixture dir does not exist, so `git` is skipped (see the tempdir-repo
    // test below for the git path)
    let out = run_fixture(include_str!("fixtures/full.json"), &["--width", "200"]).unwrap();
    let visible = strip_ansi(&out.bar);
    assert_eq!(
        visible,
        "░▒▓ \u{f4f5} \u{e0b0} apps/emissions \u{e0b1} \u{f16a6} opus 4.8 \u{e0b0} \
         15,500 tok \u{e0b0} $0.71 \u{e0b1} 5h 77% · 7d 59% \u{e0b0} 1h 12m \u{e0b0} high \u{e0b0}"
    );
}

#[test]
fn compatible_mode_renders_without_nerd_font_glyphs() {
    let out = run_fixture(
        include_str!("fixtures/full.json"),
        &["--mode", "compatible", "--width", "200"],
    )
    .unwrap();
    let visible = strip_ansi(&out.bar);
    assert!(
        !visible
            .chars()
            .any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)
                || ('\u{f0000}'..='\u{10ffff}').contains(&c)),
        "no private-use glyphs in compatible mode: {visible:?}"
    );
    assert!(visible.contains("\u{2733}"), "plain logo: {visible:?}");
    assert!(visible.contains("opus 4.8"), "{visible:?}");
}

#[test]
fn modules_right_pads_the_bar_to_the_terminal_width() {
    let out = run_fixture(
        include_str!("fixtures/full.json"),
        &[
            "--modules",
            "logo",
            "--modules-right",
            "model",
            "--mode",
            "flat",
            "--width",
            "40",
        ],
    )
    .unwrap();
    let visible = strip_ansi(&out.bar);
    assert_eq!(visible.chars().count(), 40, "{visible:?}");
    assert!(visible.starts_with(" \u{f4f5} "), "{visible:?}");
    assert!(visible.ends_with(" \u{f16a6} opus 4.8 "), "{visible:?}");
}

#[test]
fn modules_right_keeps_one_space_when_the_bar_overflows() {
    let out = run_fixture(
        include_str!("fixtures/full.json"),
        &[
            "--modules",
            "logo",
            "--modules-right",
            "model",
            "--mode",
            "flat",
            "--width",
            "5",
        ],
    )
    .unwrap();
    assert_eq!(strip_ansi(&out.bar), " \u{f4f5}   \u{f16a6} opus 4.8 ");
}

#[test]
fn context_over_80k_tokens_gets_the_warn_background() {
    let payload = r#"{"context_window":{"total_input_tokens":90000}}"#;
    let out = run_fixture(payload, &["--modules", "context", "--mode", "flat"]).unwrap();
    // catppuccin-mocha context_warn bg: #fab387
    assert!(out.bar.contains("\x1b[48;2;250;179;135m"), "{:?}", out.bar);
}

#[test]
fn context_over_125k_tokens_gets_the_alert_background() {
    let payload = r#"{"context_window":{"total_input_tokens":130000}}"#;
    let out = run_fixture(payload, &["--modules", "context", "--mode", "flat"]).unwrap();
    // catppuccin-mocha context_alert bg: #f38ba8
    assert!(out.bar.contains("\x1b[48;2;243;139;168m"), "{:?}", out.bar);
}

#[test]
fn usage_renders_remaining_rate_limit_budget() {
    let out = run_fixture(
        include_str!("fixtures/full.json"),
        &["--modules", "usage", "--mode", "flat"],
    )
    .unwrap();
    assert_eq!(strip_ansi(&out.bar), " 5h 77% · 7d 59% ");
}

#[test]
fn modules_flag_selects_and_orders_segments() {
    let out = run_fixture(
        include_str!("fixtures/full.json"),
        &["--modules", "cost,model", "--mode", "flat"],
    )
    .unwrap();
    assert_eq!(strip_ansi(&out.bar), " $0.71  \u{f16a6} opus 4.8 ");
}

#[test]
fn bare_theme_name_resolves_from_the_home_config_directory() {
    let home = tempfile::tempdir().unwrap();
    let theme_dir = home.path().join(".config/powerline-claude/themes/my-theme");
    std::fs::create_dir_all(&theme_dir).unwrap();
    std::fs::write(theme_dir.join("theme.yaml"), "model: { bg: \"#123456\" }\n").unwrap();
    let env = Env {
        home: home.path().to_str().unwrap().to_string(),
        columns: None,
    };
    let out = run(
        include_str!("fixtures/full.json"),
        &cli(&[
            "--theme",
            "my-theme",
            "--modules",
            "model",
            "--mode",
            "flat",
        ]),
        &env,
        || None,
    )
    .unwrap();
    assert!(out.bar.contains("\x1b[48;2;18;52;86m"), "{:?}", out.bar);
}

#[test]
fn unknown_theme_is_a_readable_error() {
    let err = run_fixture(include_str!("fixtures/full.json"), &["--theme", "nope"]).unwrap_err();
    assert!(err.contains("unknown theme 'nope'"), "{err}");
}

#[test]
fn unknown_module_is_a_readable_error() {
    let err = run_fixture(include_str!("fixtures/full.json"), &["--modules", "bogus"]).unwrap_err();
    assert!(err.contains("unknown module 'bogus'"), "{err}");
}

#[test]
fn garbage_payload_is_an_error_not_a_panic() {
    assert!(run_fixture("not json", &[]).is_err());
}

#[test]
fn git_segment_flows_through_run_with_line_churn() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    let payload = format!(
        r#"{{"workspace": {{"current_dir": "{}"}},
             "cost": {{"total_lines_added": 156, "total_lines_removed": 23}}}}"#,
        repo.path().display()
    );
    let out = run(
        &payload,
        &cli(&["--modules", "git", "--mode", "flat"]),
        &env(),
        || None,
    )
    .unwrap();
    assert_eq!(strip_ansi(&out.bar), " \u{e0a0} main +156 -23 ");
}

// --- progress output ---

#[test]
fn full_payload_emits_the_progress_sequence() {
    let out = run_fixture(include_str!("fixtures/full.json"), &[]).unwrap();
    // 15500 / 200000 = 7% used -> normal state, fill 7*100/80 = 8
    assert_eq!(out.progress.as_deref(), Some("\x1b]9;4;1;8\x07"));
}

#[test]
fn no_progress_flag_suppresses_the_sequence() {
    let out = run_fixture(include_str!("fixtures/full.json"), &["--no-progress"]).unwrap();
    assert_eq!(out.progress, None);
}

#[test]
fn payload_without_context_numbers_emits_no_progress() {
    // minimal.json: current_usage is null and total_input_tokens is 0
    let out = run_fixture(include_str!("fixtures/minimal.json"), &[]).unwrap();
    assert_eq!(out.progress, None);
}

// --- width ---

#[test]
fn width_prefers_flag_then_columns_then_tty_probe_then_default() {
    use powerline_claude::resolve_width;
    assert_eq!(resolve_width(Some(120), Some("80"), || Some(60)), 120);
    assert_eq!(resolve_width(None, Some("80"), || Some(60)), 80);
    assert_eq!(resolve_width(None, None, || Some(60)), 60);
    assert_eq!(resolve_width(None, None, || None), 200);
    // unparsable COLUMNS falls through to the probe
    assert_eq!(resolve_width(None, Some("not a number"), || Some(60)), 60);
}

#[test]
fn columns_env_drives_dir_truncation_through_run() {
    let narrow = Env {
        home: "/home/user".to_string(),
        columns: Some("79".to_string()),
    };
    let out = run(
        include_str!("fixtures/full.json"),
        &cli(&["--modules", "dir", "--mode", "flat"]),
        &narrow,
        || None,
    )
    .unwrap();
    assert_eq!(strip_ansi(&out.bar), " emissions ");
}
