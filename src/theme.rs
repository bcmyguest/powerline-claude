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

/// One semantic color family: what a palette (and a custom `theme.yaml`)
/// actually defines. Segments pick their fg and bg from these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Claude,
    Directory,
    Git,
    Model,
    Context,
    Cost,
}

impl Family {
    pub const ALL: [Family; 6] = [
        Family::Claude,
        Family::Directory,
        Family::Git,
        Family::Model,
        Family::Context,
        Family::Cost,
    ];
}

/// Const constructor for palette entries: `(fg, bg)` as hex words.
const fn sc(fg: u32, bg: u32) -> SegmentColors {
    SegmentColors {
        fg: Rgb::hex(fg),
        bg: Rgb::hex(bg),
    }
}

/// One vendored palette: the semantic fg/bg values from the upstream
/// `palettes/*.toml` files, indexed by `Family` (same order as `Family::ALL`:
/// claude, directory, git, model, context, cost).
#[derive(Debug)]
struct Palette {
    name: &'static str,
    colors: [SegmentColors; Family::ALL.len()],
}

const PALETTES: &[Palette] = &[
    Palette {
        name: "catppuccin-mocha",
        colors: [
            sc(0xd97757, 0x313244), // claude
            sc(0x89dceb, 0x1e1e2e), // directory
            sc(0xeba0ac, 0x313244), // git
            sc(0xb4befe, 0x1e1e2e), // model
            sc(0xfab387, 0x313244), // context
            sc(0xa6e3a1, 0x45475a), // cost
        ],
    },
    Palette {
        name: "catppuccin-frappe",
        colors: [
            sc(0xd97757, 0xeff1f5),
            sc(0x04a5e5, 0xeff1f5),
            sc(0xdd7878, 0xccd0da),
            sc(0x8839ef, 0xeff1f5),
            sc(0xdf8e1d, 0xccd0da),
            sc(0x40a02b, 0xbcc0cc),
        ],
    },
    Palette {
        name: "dracula",
        colors: [
            sc(0xd97757, 0x44475a),
            sc(0x8be9fd, 0x282a36),
            sc(0xbd93f9, 0x44475a),
            sc(0x8be9fd, 0x282a36),
            sc(0xffb86c, 0x44475a),
            sc(0x50fa7b, 0x4d4f68),
        ],
    },
    Palette {
        name: "gruvbox-dark",
        colors: [
            sc(0xd97757, 0x282828),
            sc(0x83a598, 0x282828),
            sc(0xb16286, 0x3c3836),
            sc(0x458588, 0x282828),
            sc(0xd79921, 0x3c3836),
            sc(0x689d6a, 0x504945),
        ],
    },
    Palette {
        name: "nord",
        colors: [
            sc(0xd97757, 0x2e3440),
            sc(0x88c0d0, 0x2e3440),
            sc(0xb48ead, 0x3b4252),
            sc(0x5e81ac, 0x2e3440),
            sc(0x8fbcbb, 0x3b4252),
            sc(0xa3be8c, 0x434c5e),
        ],
    },
    Palette {
        name: "tokyonight",
        colors: [
            sc(0x090c0c, 0xa3aed2),
            sc(0xe3e5e5, 0x769ff0),
            sc(0x769ff0, 0x394260),
            sc(0x769ff0, 0x212736),
            sc(0xa0a9cb, 0x1d2230),
            sc(0xc0caf5, 0x414868),
        ],
    },
];

#[derive(Debug, Deserialize, Default)]
struct RawFamily {
    fg: Option<String>,
    bg: Option<String>,
}

/// The serde adapter facing `theme.yaml`: the one place the six family
/// names are deliberately spelled out, because they are the file format.
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

impl RawTheme {
    fn take_family(&mut self, family: Family) -> Option<RawFamily> {
        match family {
            Family::Claude => self.claude.take(),
            Family::Directory => self.directory.take(),
            Family::Git => self.git.take(),
            Family::Model => self.model.take(),
            Family::Context => self.context.take(),
            Family::Cost => self.cost.take(),
        }
    }
}

/// A fully resolved theme: owned so it can come from either a vendored
/// preset or a loaded custom `theme.yaml`.
#[derive(Debug, Clone)]
pub struct Theme {
    name: String,
    colors: [SegmentColors; Family::ALL.len()],
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_preset(&PALETTES[0])
    }
}

impl Theme {
    fn from_preset(preset: &Palette) -> Self {
        Self {
            name: preset.name.to_string(),
            colors: preset.colors,
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
                let available: Vec<&str> = Self::builtin_names().collect();
                format!(
                    "unknown theme '{name}', available: {}",
                    available.join(", ")
                )
            })
    }

    /// Names of the vendored palettes, in listing order.
    pub fn builtin_names() -> impl Iterator<Item = &'static str> {
        PALETTES.iter().map(|palette| palette.name)
    }

    fn from_dir(dir: &Path) -> Result<Self, String> {
        let yaml_path = dir.join("theme.yaml");
        let contents = std::fs::read_to_string(&yaml_path)
            .map_err(|e| format!("failed to read '{}': {e}", yaml_path.display()))?;
        let mut raw: RawTheme = serde_norway::from_str(&contents)
            .map_err(|e| format!("failed to parse '{}': {e}", yaml_path.display()))?;

        let mut colors = PALETTES[0].colors;
        for family in Family::ALL {
            let overrides = raw.take_family(family).unwrap_or_default();
            let slot = &mut colors[family as usize];
            if let Some(fg) = overrides.fg {
                slot.fg = Rgb::parse_hex(&fg)?;
            }
            if let Some(bg) = overrides.bg {
                slot.bg = Rgb::parse_hex(&bg)?;
            }
        }

        let name = raw.name.unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "custom".to_string())
        });

        Ok(Self { name, colors })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The resolved fg/bg pair for one family.
    pub fn family(&self, family: Family) -> SegmentColors {
        self.colors[family as usize]
    }

    pub fn colors(&self, kind: SegmentKind) -> SegmentColors {
        match kind {
            SegmentKind::Logo => self.family(Family::Claude),
            SegmentKind::Dir => self.family(Family::Directory),
            SegmentKind::Git => self.family(Family::Git),
            SegmentKind::Model => self.family(Family::Model),
            SegmentKind::Context => self.family(Family::Context),
            SegmentKind::Cost => self.family(Family::Cost),
            SegmentKind::Stats => SegmentColors {
                fg: self.family(Family::Cost).fg,
                bg: self.family(Family::Context).bg,
            },
            SegmentKind::Effort => self.family(Family::Model),
        }
    }
}
