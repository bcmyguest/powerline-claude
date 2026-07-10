//! Color themes, ported from the starship-claude palette files.
//!
//! A palette defines fg/bg pairs per semantic family (claude, directory, git,
//! model, context, cost). The two segments that have no upstream family
//! (stats, effort) reuse existing families so every theme stays coherent:
//! stats renders with the cost fg on the context bg, effort with the model
//! colors.
//!
//! A custom theme is a directory containing a `theme.yaml` with any subset
//! of the six families below (each an optional `fg`/`bg` hex pair); anything
//! left unspecified falls back to the catppuccin-mocha value for that slot.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn hex(value: u32) -> Self {
        Self {
            r: (value >> 16) as u8,
            g: (value >> 8) as u8,
            b: value as u8,
        }
    }

    /// Parses a `#rrggbb` or `rrggbb` hex color string.
    pub fn parse_hex(value: &str) -> Result<Self, String> {
        let digits = value.strip_prefix('#').unwrap_or(value);
        if digits.len() != 6 {
            return Err(format!(
                "invalid color '{value}': expected 6 hex digits (e.g. '#d97757')"
            ));
        }
        u32::from_str_radix(digits, 16)
            .map(Self::hex)
            .map_err(|_| format!("invalid color '{value}': not a valid hex value"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentColors {
    pub fg: Rgb,
    pub bg: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Logo,
    Dir,
    Git,
    Model,
    Context,
    Cost,
    Stats,
    Effort,
}

/// One vendored palette: the semantic fg/bg values from the upstream
/// `palettes/*.toml` files.
#[derive(Debug)]
struct Palette {
    name: &'static str,
    claude: (u32, u32),
    directory: (u32, u32),
    git: (u32, u32),
    model: (u32, u32),
    context: (u32, u32),
    cost: (u32, u32),
}

const PALETTES: &[Palette] = &[
    Palette {
        name: "catppuccin-mocha",
        claude: (0xd97757, 0x313244),
        directory: (0x89dceb, 0x1e1e2e),
        git: (0xeba0ac, 0x313244),
        model: (0xb4befe, 0x1e1e2e),
        context: (0xfab387, 0x313244),
        cost: (0xa6e3a1, 0x45475a),
    },
    Palette {
        name: "catppuccin-frappe",
        claude: (0xd97757, 0xeff1f5),
        directory: (0x04a5e5, 0xeff1f5),
        git: (0xdd7878, 0xccd0da),
        model: (0x8839ef, 0xeff1f5),
        context: (0xdf8e1d, 0xccd0da),
        cost: (0x40a02b, 0xbcc0cc),
    },
    Palette {
        name: "dracula",
        claude: (0xd97757, 0x44475a),
        directory: (0x8be9fd, 0x282a36),
        git: (0xbd93f9, 0x44475a),
        model: (0x8be9fd, 0x282a36),
        context: (0xffb86c, 0x44475a),
        cost: (0x50fa7b, 0x4d4f68),
    },
    Palette {
        name: "gruvbox-dark",
        claude: (0xd97757, 0x282828),
        directory: (0x83a598, 0x282828),
        git: (0xb16286, 0x3c3836),
        model: (0x458588, 0x282828),
        context: (0xd79921, 0x3c3836),
        cost: (0x689d6a, 0x504945),
    },
    Palette {
        name: "nord",
        claude: (0xd97757, 0x2e3440),
        directory: (0x88c0d0, 0x2e3440),
        git: (0xb48ead, 0x3b4252),
        model: (0x5e81ac, 0x2e3440),
        context: (0x8fbcbb, 0x3b4252),
        cost: (0xa3be8c, 0x434c5e),
    },
    Palette {
        name: "tokyonight",
        claude: (0x090c0c, 0xa3aed2),
        directory: (0xe3e5e5, 0x769ff0),
        git: (0x769ff0, 0x394260),
        model: (0x769ff0, 0x212736),
        context: (0xa0a9cb, 0x1d2230),
        cost: (0xc0caf5, 0x414868),
    },
];

#[derive(Debug, Deserialize, Default)]
struct RawFamily {
    fg: Option<String>,
    bg: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTheme {
    name: Option<String>,
    claude: Option<RawFamily>,
    directory: Option<RawFamily>,
    git: Option<RawFamily>,
    model: Option<RawFamily>,
    context: Option<RawFamily>,
    cost: Option<RawFamily>,
}

/// A fully resolved theme: owned so it can come from either a vendored
/// preset or a loaded custom `theme.yaml`.
#[derive(Debug, Clone)]
pub struct Theme {
    name: String,
    claude: SegmentColors,
    directory: SegmentColors,
    git: SegmentColors,
    model: SegmentColors,
    context: SegmentColors,
    cost: SegmentColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_preset(&PALETTES[0])
    }
}

impl Theme {
    fn from_preset(preset: &Palette) -> Self {
        let colors = |pair: (u32, u32)| SegmentColors {
            fg: Rgb::hex(pair.0),
            bg: Rgb::hex(pair.1),
        };
        Self {
            name: preset.name.to_string(),
            claude: colors(preset.claude),
            directory: colors(preset.directory),
            git: colors(preset.git),
            model: colors(preset.model),
            context: colors(preset.context),
            cost: colors(preset.cost),
        }
    }

    pub fn by_name(name: &str) -> Result<Self, String> {
        let path = Path::new(name);
        if path.is_dir() {
            return Self::from_dir(path);
        }
        PALETTES
            .iter()
            .find(|palette| palette.name == name)
            .map(Self::from_preset)
            .ok_or_else(|| {
                let available: Vec<&str> = PALETTES.iter().map(|p| p.name).collect();
                format!(
                    "unknown theme '{name}', available: {}",
                    available.join(", ")
                )
            })
    }

    fn from_dir(dir: &Path) -> Result<Self, String> {
        let yaml_path = dir.join("theme.yaml");
        let contents = std::fs::read_to_string(&yaml_path)
            .map_err(|e| format!("failed to read '{}': {e}", yaml_path.display()))?;
        let raw: RawTheme = serde_norway::from_str(&contents)
            .map_err(|e| format!("failed to parse '{}': {e}", yaml_path.display()))?;

        let defaults = &PALETTES[0];
        let resolve =
            |default: (u32, u32), family: Option<RawFamily>| -> Result<SegmentColors, String> {
                let (default_fg, default_bg) = default;
                let family = family.unwrap_or_default();
                let fg = match family.fg {
                    Some(s) => Rgb::parse_hex(&s)?,
                    None => Rgb::hex(default_fg),
                };
                let bg = match family.bg {
                    Some(s) => Rgb::parse_hex(&s)?,
                    None => Rgb::hex(default_bg),
                };
                Ok(SegmentColors { fg, bg })
            };

        let name = raw.name.unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "custom".to_string())
        });

        Ok(Self {
            name,
            claude: resolve(defaults.claude, raw.claude)?,
            directory: resolve(defaults.directory, raw.directory)?,
            git: resolve(defaults.git, raw.git)?,
            model: resolve(defaults.model, raw.model)?,
            context: resolve(defaults.context, raw.context)?,
            cost: resolve(defaults.cost, raw.cost)?,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn colors(&self, kind: SegmentKind) -> SegmentColors {
        match kind {
            SegmentKind::Logo => self.claude,
            SegmentKind::Dir => self.directory,
            SegmentKind::Git => self.git,
            SegmentKind::Model => self.model,
            SegmentKind::Context => self.context,
            SegmentKind::Cost => self.cost,
            SegmentKind::Stats => SegmentColors {
                fg: self.cost.fg,
                bg: self.context.bg,
            },
            SegmentKind::Effort => self.model,
        }
    }
}
