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
    assert_eq!(cli.modules, "logo,dir,git,model,context,cost,stats,effort");
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
         15,500 tok \u{e0b0} $0.71 \u{e0b0} 1h 12m \u{e0b0} high \u{e0b0}"
    );
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
