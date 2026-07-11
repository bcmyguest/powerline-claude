use powerline_claude::theme::{Family, Rgb, SegmentColors, Theme};

fn write_theme_yaml(dir: &std::path::Path, contents: &str) {
    std::fs::write(dir.join("theme.yaml"), contents).unwrap();
}

#[test]
fn default_theme_is_catppuccin_mocha() {
    let theme = Theme::default();
    assert_eq!(theme.name(), "catppuccin-mocha");
}

#[test]
fn resolves_all_six_palettes_by_name() {
    for name in [
        "catppuccin-mocha",
        "catppuccin-frappe",
        "dracula",
        "gruvbox-dark",
        "nord",
        "tokyonight",
    ] {
        let theme = Theme::by_name(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(theme.name(), name);
    }
}

#[test]
fn unknown_theme_error_lists_available_names() {
    let err = Theme::by_name("solarized").unwrap_err();
    assert!(err.contains("solarized"), "names the bad input: {err}");
    assert!(
        err.contains("catppuccin-mocha"),
        "lists valid themes: {err}"
    );
}

#[test]
fn mocha_cost_colors_match_vendored_palette() {
    let colors = Theme::default().family(Family::Cost);
    assert_eq!(colors.fg, Rgb::hex(0xa6e3a1));
    assert_eq!(colors.bg, Rgb::hex(0x45475a));
}

#[test]
fn mocha_git_uses_branch_fg_on_git_bg() {
    let colors = Theme::default().family(Family::Git);
    assert_eq!(colors.fg, Rgb::hex(0xeba0ac));
    assert_eq!(colors.bg, Rgb::hex(0x313244));
}

#[test]
fn every_family_resolves_in_every_builtin() {
    for name in Theme::builtin_names() {
        let theme = Theme::by_name(name).unwrap();
        for family in Family::ALL {
            let _ = theme.family(family); // no panic on any slot
        }
    }
}

#[test]
fn rgb_decomposes_into_channels() {
    let rgb = Rgb::hex(0xd97757);
    assert_eq!((rgb.r, rgb.g, rgb.b), (0xd9, 0x77, 0x57));
}

#[test]
fn parse_hex_accepts_leading_hash() {
    let rgb = Rgb::parse_hex("#d97757").unwrap();
    assert_eq!(rgb, Rgb::hex(0xd97757));
}

#[test]
fn parse_hex_accepts_no_leading_hash() {
    let rgb = Rgb::parse_hex("d97757").unwrap();
    assert_eq!(rgb, Rgb::hex(0xd97757));
}

#[test]
fn parse_hex_rejects_wrong_length() {
    let err = Rgb::parse_hex("#d977").unwrap_err();
    assert!(err.contains("d977"), "names the bad input: {err}");
}

#[test]
fn parse_hex_rejects_non_hex_chars() {
    let err = Rgb::parse_hex("#gggggg").unwrap_err();
    assert!(err.contains("gggggg"), "names the bad input: {err}");
}

#[test]
fn custom_theme_dir_with_full_override_returns_exact_colors() {
    let dir = tempfile::tempdir().unwrap();
    write_theme_yaml(
        dir.path(),
        r##"
name: my-custom
claude: { fg: "#111111", bg: "#222222" }
directory: { fg: "#333333", bg: "#444444" }
git: { fg: "#555555", bg: "#666666" }
model: { fg: "#777777", bg: "#888888" }
context: { fg: "#999999", bg: "#aaaaaa" }
cost: { fg: "#bbbbbb", bg: "#cccccc" }
"##,
    );

    let theme = Theme::by_name(dir.path().to_str().unwrap()).unwrap();

    assert_eq!(
        theme.family(Family::Claude),
        SegmentColors {
            fg: Rgb::hex(0x111111),
            bg: Rgb::hex(0x222222)
        }
    );
    assert_eq!(
        theme.family(Family::Cost),
        SegmentColors {
            fg: Rgb::hex(0xbbbbbb),
            bg: Rgb::hex(0xcccccc)
        }
    );
}

#[test]
fn custom_theme_dir_partial_family_falls_back_to_mocha() {
    let dir = tempfile::tempdir().unwrap();
    // Only claude.fg is overridden; everything else should fall back to
    // catppuccin-mocha, including claude.bg.
    write_theme_yaml(dir.path(), "claude: { fg: \"#123456\" }\n");

    let theme = Theme::by_name(dir.path().to_str().unwrap()).unwrap();
    let mocha = Theme::default();

    assert_eq!(
        theme.family(Family::Claude),
        SegmentColors {
            fg: Rgb::hex(0x123456),
            bg: mocha.family(Family::Claude).bg,
        }
    );
    assert_eq!(theme.family(Family::Git), mocha.family(Family::Git));
    assert_eq!(theme.family(Family::Cost), mocha.family(Family::Cost));
}

#[test]
fn custom_theme_dir_missing_yaml_file_errors_with_path() {
    let dir = tempfile::tempdir().unwrap();

    let err = Theme::by_name(dir.path().to_str().unwrap()).unwrap_err();

    assert!(err.contains("theme.yaml"), "names the expected file: {err}");
}

#[test]
fn custom_theme_dir_invalid_hex_color_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_theme_yaml(dir.path(), "claude: { fg: \"not-a-color\" }\n");

    let err = Theme::by_name(dir.path().to_str().unwrap()).unwrap_err();

    assert!(err.contains("not-a-color"), "names the bad value: {err}");
}

#[test]
fn custom_theme_name_uses_explicit_name_field() {
    let dir = tempfile::tempdir().unwrap();
    write_theme_yaml(dir.path(), "name: my-explicit-name\n");

    let theme = Theme::by_name(dir.path().to_str().unwrap()).unwrap();

    assert_eq!(theme.name(), "my-explicit-name");
}

#[test]
fn custom_theme_name_falls_back_to_directory_basename() {
    let dir = tempfile::tempdir().unwrap();
    let named_dir = dir.path().join("my-dir-name");
    std::fs::create_dir(&named_dir).unwrap();
    write_theme_yaml(&named_dir, "claude: { fg: \"#111111\" }\n");

    let theme = Theme::by_name(named_dir.to_str().unwrap()).unwrap();

    assert_eq!(theme.name(), "my-dir-name");
}
