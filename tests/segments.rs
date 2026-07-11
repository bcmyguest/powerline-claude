use std::fs;

use powerline_claude::payload::Payload;
use powerline_claude::segments::{
    Module, context_family, format_cost, format_duration, format_model, format_tokens,
    format_usage, git_branch, parse_modules, segment_texts, truncate_dir,
};
use powerline_claude::theme::{Family, Rgb, Theme};

// --- registry: colors ---

#[test]
fn stats_effort_and_usage_borrow_palette_families() {
    let theme = Theme::default(); // catppuccin-mocha
    // stats: cost fg on the context bg (keeps the alternating-bg rhythm)
    let stats = Module::Stats.colors(&theme, None);
    assert_eq!(stats.fg, Rgb::hex(0xa6e3a1));
    assert_eq!(stats.bg, Rgb::hex(0x313244));
    // effort: model colors
    let effort = Module::Effort.colors(&theme, None);
    assert_eq!(effort.fg, Rgb::hex(0xb4befe));
    assert_eq!(effort.bg, Rgb::hex(0x1e1e2e));
    // usage: cost colors
    assert_eq!(
        Module::Usage.colors(&theme, None),
        Module::Cost.colors(&theme, None)
    );
}

#[test]
fn context_colors_shift_with_the_token_count() {
    let theme = Theme::default();
    let calm = Module::Context.colors(&theme, Some(15_000));
    assert_eq!(calm.bg, theme.family(Family::Context).bg);
    let warn = Module::Context.colors(&theme, Some(90_000));
    assert_eq!(warn.bg, theme.family(Family::ContextWarn).bg);
    let alert = Module::Context.colors(&theme, Some(130_000));
    assert_eq!(alert.bg, theme.family(Family::ContextAlert).bg);
}

#[test]
fn default_list_is_the_cli_default() {
    assert_eq!(
        Module::default_list(),
        "logo,dir,git,model,context,cost,usage,stats,effort"
    );
}

// --- model ---

#[test]
fn model_gets_family_icon_and_lowercase_name() {
    assert_eq!(format_model("Opus 4.8"), "\u{f16a6} opus 4.8");
    assert_eq!(format_model("Sonnet 5"), "\u{f06a9} sonnet 5");
    assert_eq!(format_model("Haiku 4.5"), "\u{ee0d} haiku 4.5");
}

#[test]
fn unknown_model_family_falls_back_to_sonnet_icon() {
    assert_eq!(format_model("Fable 5"), "\u{f06a9} fable 5");
}

// --- context tokens ---

#[test]
fn tokens_render_with_thousands_separators() {
    assert_eq!(format_tokens(Some(15500)), "15,500 tok");
    assert_eq!(format_tokens(Some(1_234_567)), "1,234,567 tok");
    assert_eq!(format_tokens(Some(999)), "999 tok");
}

#[test]
fn missing_or_zero_tokens_render_placeholder() {
    assert_eq!(format_tokens(None), "~~ tok");
    assert_eq!(format_tokens(Some(0)), "~~ tok");
}

// --- context thresholds ---

#[test]
fn context_turns_orange_at_80k_and_red_at_125k() {
    assert_eq!(context_family(Some(79_999)), Family::Context);
    assert_eq!(context_family(Some(80_000)), Family::ContextWarn);
    assert_eq!(context_family(Some(124_999)), Family::ContextWarn);
    assert_eq!(context_family(Some(125_000)), Family::ContextAlert);
}

#[test]
fn missing_tokens_keep_the_normal_context_family() {
    assert_eq!(context_family(None), Family::Context);
}

// --- rate-limit usage ---

#[test]
fn usage_shows_remaining_percentage_per_window() {
    assert_eq!(
        format_usage(Some(23.5), Some(41.2)),
        Some("5h 77% · 7d 59%".to_string())
    );
    assert_eq!(format_usage(Some(23.5), None), Some("5h 77%".to_string()));
    assert_eq!(format_usage(None, Some(99.9)), Some("7d 0%".to_string()));
}

#[test]
fn usage_hides_without_rate_limit_data_and_never_goes_negative() {
    assert_eq!(format_usage(None, None), None);
    assert_eq!(format_usage(Some(120.0), None), Some("5h 0%".to_string()));
}

// --- cost ---

#[test]
fn cost_renders_as_dollars_with_two_decimals() {
    assert_eq!(format_cost(0.71234), "$0.71");
    assert_eq!(format_cost(0.0), "$0.00");
    assert_eq!(format_cost(12.999), "$13.00");
}

// --- duration ---

#[test]
fn duration_scales_units() {
    assert_eq!(format_duration(45_000), "45s");
    assert_eq!(format_duration(720_000), "12m");
    assert_eq!(format_duration(4_335_000), "1h 12m");
    assert_eq!(format_duration(90_061_000), "25h 1m");
}

// --- dir ---

#[test]
fn dir_shows_home_as_tilde() {
    assert_eq!(truncate_dir("/home/user", "/home/user", 200), "~");
}

#[test]
fn dir_keeps_last_two_components_with_ellipsis() {
    assert_eq!(
        truncate_dir(
            "/home/user/projects/backend/apps/emissions",
            "/home/user",
            200
        ),
        "apps/emissions"
    );
}

#[test]
fn short_paths_are_not_truncated() {
    assert_eq!(
        truncate_dir("/home/user/projects", "/home/user", 200),
        "~/projects"
    );
}

#[test]
fn narrow_terminals_keep_only_the_last_component() {
    assert_eq!(
        truncate_dir(
            "/home/user/projects/backend/apps/emissions",
            "/home/user",
            79
        ),
        "emissions"
    );
}

// --- git branch (no subprocess: reads .git/HEAD) ---

#[test]
fn branch_read_from_git_head_ref() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    assert_eq!(git_branch(repo.path()), Some("main".to_string()));
}

#[test]
fn branch_found_from_nested_subdirectory() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join(".git/HEAD"), "ref: refs/heads/feat/x\n").unwrap();
    let nested = repo.path().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    assert_eq!(git_branch(&nested), Some("feat/x".to_string()));
}

#[test]
fn detached_head_shows_short_hash() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(
        repo.path().join(".git/HEAD"),
        "20ece3f7aabbccddeeff00112233445566778899\n",
    )
    .unwrap();
    assert_eq!(git_branch(repo.path()), Some("20ece3f7".to_string()));
}

#[test]
fn linked_worktree_gitfile_is_followed() {
    let main = tempfile::tempdir().unwrap();
    let gitdir = main.path().join("repo/.git/worktrees/wt");
    fs::create_dir_all(&gitdir).unwrap();
    fs::write(gitdir.join("HEAD"), "ref: refs/heads/wt-branch\n").unwrap();

    let worktree = tempfile::tempdir().unwrap();
    fs::write(
        worktree.path().join(".git"),
        format!("gitdir: {}\n", gitdir.display()),
    )
    .unwrap();
    assert_eq!(git_branch(worktree.path()), Some("wt-branch".to_string()));
}

#[test]
fn no_repository_means_no_branch() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(git_branch(dir.path()), None);
}

// --- module list parsing ---

#[test]
fn parses_module_list_in_given_order() {
    let modules = parse_modules("cost,model").unwrap();
    assert_eq!(modules, vec![Module::Cost, Module::Model]);
}

#[test]
fn default_module_order_matches_todays_bar() {
    assert_eq!(
        Module::default_order(),
        vec![
            Module::Logo,
            Module::Dir,
            Module::Git,
            Module::Model,
            Module::Context,
            Module::Cost,
            Module::Usage,
            Module::Stats,
            Module::Effort,
        ]
    );
}

#[test]
fn unknown_module_error_names_it_and_lists_valid_ones() {
    let err = parse_modules("logo,bogus").unwrap_err();
    assert!(err.contains("bogus"), "{err}");
    assert!(err.contains("context"), "{err}");
}

// --- composition: payload -> segment texts ---

fn payload(json: &str) -> Payload {
    Payload::from_json(json).unwrap()
}

fn tempdir_repo(branch: &str) -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(
        repo.path().join(".git/HEAD"),
        format!("ref: refs/heads/{branch}\n"),
    )
    .unwrap();
    repo
}

#[test]
fn git_segment_appends_churn_when_both_line_counts_exist() {
    let repo = tempdir_repo("main");
    let p = payload(&format!(
        r#"{{"workspace": {{"current_dir": "{}"}},
             "cost": {{"total_lines_added": 5, "total_lines_removed": 2}}}}"#,
        repo.path().display()
    ));
    let texts = segment_texts(&p, &[Module::Git], 200, "/home/user");
    assert_eq!(
        texts,
        vec![(Module::Git, "\u{e0a0} main +5 -2".to_string())]
    );
}

#[test]
fn git_segment_omits_churn_when_a_line_count_is_missing() {
    let repo = tempdir_repo("main");
    let p = payload(&format!(
        r#"{{"workspace": {{"current_dir": "{}"}},
             "cost": {{"total_lines_added": 5}}}}"#,
        repo.path().display()
    ));
    let texts = segment_texts(&p, &[Module::Git], 200, "/home/user");
    assert_eq!(texts, vec![(Module::Git, "\u{e0a0} main".to_string())]);
}

#[test]
fn each_optional_segment_drops_when_its_data_is_absent() {
    // model only: dir, git, cost, stats, and effort must all drop; logo
    // always renders and context shows its placeholder.
    let p = payload(r#"{"model": {"display_name": "Sonnet 5"}}"#);
    let texts = segment_texts(&p, &Module::default_order(), 200, "/home/user");
    let modules: Vec<Module> = texts.iter().map(|(module, _)| *module).collect();
    assert_eq!(modules, vec![Module::Logo, Module::Model, Module::Context]);
}

#[test]
fn segments_skip_absent_data() {
    // minimal payload: no cost, no effort, dir exists but is no git repo
    let payload = Payload::from_json(include_str!("fixtures/minimal.json")).unwrap();
    let texts = powerline_claude::segments::segment_texts(
        &payload,
        &Module::default_order(),
        200,
        "/home/user",
    );
    let joined: Vec<&str> = texts.iter().map(|(_, t)| t.as_str()).collect();
    assert!(joined.iter().any(|t| t.contains("sonnet 5")), "{joined:?}");
    assert!(joined.iter().any(|t| t == &"~~ tok"), "{joined:?}");
    assert!(
        !joined.iter().any(|t| t.starts_with('$')),
        "cost must be skipped: {joined:?}"
    );
}
