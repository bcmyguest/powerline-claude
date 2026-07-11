//! Guards the hand-maintained doc surfaces against drifting from the code.
//!
//! The README flags table and the plugin's `/powerline-claude:configure`
//! command both hand-list the valid modules, themes, and modes (users copy
//! flags from them into settings.json), and CLAUDE.md asks every flag change
//! to touch all three surfaces. These tests turn that checklist into CI.

use powerline_claude::payload::Payload;
use powerline_claude::render::Mode;
use powerline_claude::segments::Module;
use powerline_claude::theme::Theme;

fn readme() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap()
}

fn configure_command() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/plugin/commands/configure.md"
    ))
    .unwrap()
}

#[test]
fn docs_list_the_full_default_module_string() {
    // The exact default --modules value appears verbatim in both docs, so a
    // registry change (new module, reorder) fails here until they're synced.
    let default_list = Module::default_list();
    assert!(
        readme().contains(&default_list),
        "README.md must list the default --modules value '{default_list}'"
    );
    assert!(
        configure_command().contains(&default_list),
        "plugin/commands/configure.md must list the default --modules value '{default_list}'"
    );
}

#[test]
fn docs_mention_every_builtin_theme() {
    let readme = readme();
    let configure = configure_command();
    for name in Theme::builtin_names() {
        assert!(readme.contains(name), "README.md must mention theme {name}");
        assert!(
            configure.contains(name),
            "plugin/commands/configure.md must mention theme {name}"
        );
    }
}

#[test]
fn docs_mention_every_mode() {
    let readme = readme();
    let configure = configure_command();
    for name in Mode::NAMES {
        assert!(readme.contains(name), "README.md must mention mode {name}");
        assert!(
            configure.contains(name),
            "plugin/commands/configure.md must mention mode {name}"
        );
    }
}

#[test]
fn configure_command_sample_payload_still_parses() {
    // The preview step embeds a hand-written payload; keep it valid against
    // the real parser so the plugin's preview can't silently rot.
    let doc = configure_command();
    let line = doc
        .lines()
        .find(|line| line.trim_start().starts_with("echo '"))
        .expect("configure.md must keep the sample-payload preview line");
    let start = line.find("echo '").unwrap() + "echo '".len();
    let end = line.rfind('\'').unwrap();
    let json = line[start..end].replace(r#"'"$PWD"'"#, "/tmp");

    let payload = Payload::from_json(&json).expect("sample payload must deserialize");
    assert_eq!(payload.model_display_name(), Some("Opus 4.8"));
    assert_eq!(payload.dir(), Some("/tmp"));
    assert_eq!(payload.current_tokens(), Some(15500));
    assert!(payload.total_cost_usd().is_some());
}
