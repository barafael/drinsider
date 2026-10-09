//! High-contrast palettes and the flat, dense style.

use egui::{
    Color32, CornerRadius, FontId, Shadow, Stroke, Style, TextStyle, Theme, ThemePreference, Ui,
};

/// Semantic colors.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Window background.
    pub bg: Color32,
    /// Bars and popups.
    pub surface: Color32,
    /// Odd table rows.
    pub stripe: Color32,
    /// Hovered rows and keycaps.
    pub hover: Color32,
    /// Separators.
    pub border: Color32,
    /// Body text.
    pub text: Color32,
    /// Emphasized text.
    pub strong: Color32,
    /// Secondary text; still well above WCAG AA.
    pub weak: Color32,
    /// binsider orange: selection and focus.
    pub accent: Color32,
    /// Text on top of `accent`.
    pub on_accent: Color32,
    /// Field labels (binsider uses cyan).
    pub label: Color32,
    /// Success, whitespace bytes.
    pub green: Color32,
    /// Errors.
    pub red: Color32,
    /// Results, non-ASCII bytes.
    pub yellow: Color32,
    /// Blue terminal color.
    pub blue: Color32,
    /// Control bytes.
    pub magenta: Color32,
    /// Null bytes.
    pub dim: Color32,
    /// Search hit background.
    pub hit_bg: Color32,
    /// Search hit text.
    pub hit_fg: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Dark palette (default).
const DARK: Palette = Palette {
    bg: hex(0x0e1117),
    surface: hex(0x1a1f29),
    stripe: hex(0x151a22),
    hover: hex(0x283040),
    border: hex(0x3b4455),
    text: hex(0xe6edf3),
    strong: hex(0xffffff),
    weak: hex(0xa9b4c4),
    accent: hex(0xf8be75),
    on_accent: hex(0x15100a),
    label: hex(0x6fd3e8),
    green: hex(0x7ee787),
    red: hex(0xff7b72),
    yellow: hex(0xf5d76e),
    blue: hex(0x79b8ff),
    magenta: hex(0xe2a5ff),
    dim: hex(0x7d8798),
    hit_bg: hex(0xf5d76e),
    hit_fg: hex(0x000000),
};

/// Light palette.
const LIGHT: Palette = Palette {
    bg: hex(0xffffff),
    surface: hex(0xeef1f5),
    stripe: hex(0xf5f7fa),
    hover: hex(0xdfe5ee),
    border: hex(0xb9c2cf),
    text: hex(0x1a1e24),
    strong: hex(0x000000),
    weak: hex(0x4a5361),
    accent: hex(0x9c4a00),
    on_accent: hex(0xffffff),
    label: hex(0x075f74),
    green: hex(0x176b30),
    red: hex(0xb3261e),
    yellow: hex(0x7a5600),
    blue: hex(0x0b4fae),
    magenta: hex(0x7a35c9),
    dim: hex(0x6a7380),
    hit_bg: hex(0xffd84d),
    hit_fg: hex(0x000000),
};

/// Gruvbox dark (hard contrast).
const GRUVBOX_DARK: Palette = Palette {
    bg: hex(0x1d2021),
    surface: hex(0x32302f),
    stripe: hex(0x282828),
    hover: hex(0x504945),
    border: hex(0x665c54),
    text: hex(0xebdbb2),
    strong: hex(0xfbf1c7),
    weak: hex(0xbdae93),
    accent: hex(0xfe8019),
    on_accent: hex(0x1d2021),
    label: hex(0x8ec07c),
    green: hex(0xb8bb26),
    red: hex(0xfb4934),
    yellow: hex(0xfabd2f),
    blue: hex(0x83a598),
    magenta: hex(0xd3869b),
    dim: hex(0x928374),
    hit_bg: hex(0xfabd2f),
    hit_fg: hex(0x1d2021),
};

/// Gruvbox light.
const GRUVBOX_LIGHT: Palette = Palette {
    bg: hex(0xfbf1c7),
    surface: hex(0xebdbb2),
    stripe: hex(0xf2e5bc),
    hover: hex(0xd5c4a1),
    border: hex(0xbdae93),
    text: hex(0x3c3836),
    strong: hex(0x1d2021),
    weak: hex(0x665c54),
    accent: hex(0xaf3a03),
    on_accent: hex(0xfbf1c7),
    label: hex(0x427b58),
    green: hex(0x79740e),
    red: hex(0x9d0006),
    yellow: hex(0xb57614),
    blue: hex(0x076678),
    magenta: hex(0x8f3f71),
    dim: hex(0x928374),
    hit_bg: hex(0xfabd2f),
    hit_fg: hex(0x1d2021),
};

/// Solarized dark, with brighter body text.
const SOLARIZED_DARK: Palette = Palette {
    bg: hex(0x002b36),
    surface: hex(0x073642),
    stripe: hex(0x01303c),
    hover: hex(0x0e4a59),
    border: hex(0x586e75),
    text: hex(0x93a1a1),
    strong: hex(0xeee8d5),
    weak: hex(0x839496),
    accent: hex(0xb58900),
    on_accent: hex(0x002b36),
    label: hex(0x2aa198),
    green: hex(0x859900),
    red: hex(0xdc322f),
    yellow: hex(0xcb4b16),
    blue: hex(0x268bd2),
    magenta: hex(0xd33682),
    dim: hex(0x657b83),
    hit_bg: hex(0xb58900),
    hit_fg: hex(0x002b36),
};

/// Solarized light, with darkened accents for contrast.
const SOLARIZED_LIGHT: Palette = Palette {
    bg: hex(0xfdf6e3),
    surface: hex(0xeee8d5),
    stripe: hex(0xf6efdb),
    hover: hex(0xe1dac4),
    border: hex(0x93a1a1),
    text: hex(0x46585f),
    strong: hex(0x002b36),
    weak: hex(0x586e75),
    accent: hex(0xb3410f),
    on_accent: hex(0xfdf6e3),
    label: hex(0x1a7d76),
    green: hex(0x5f6e00),
    red: hex(0xc4231f),
    yellow: hex(0x8a6800),
    blue: hex(0x1f6fa8),
    magenta: hex(0xb52a6c),
    dim: hex(0x93a1a1),
    hit_bg: hex(0xf2c94c),
    hit_fg: hex(0x002b36),
};

/// Catppuccin Mocha.
const CATPPUCCIN_DARK: Palette = Palette {
    bg: hex(0x1e1e2e),
    surface: hex(0x313244),
    stripe: hex(0x24243a),
    hover: hex(0x45475a),
    border: hex(0x585b70),
    text: hex(0xcdd6f4),
    strong: hex(0xffffff),
    weak: hex(0xa6adc8),
    accent: hex(0xfab387),
    on_accent: hex(0x1e1e2e),
    label: hex(0x89dceb),
    green: hex(0xa6e3a1),
    red: hex(0xf38ba8),
    yellow: hex(0xf9e2af),
    blue: hex(0x89b4fa),
    magenta: hex(0xcba6f7),
    dim: hex(0x7f849c),
    hit_bg: hex(0xf9e2af),
    hit_fg: hex(0x1e1e2e),
};

/// Catppuccin Latte, with darkened accents for contrast.
const CATPPUCCIN_LIGHT: Palette = Palette {
    bg: hex(0xeff1f5),
    surface: hex(0xdce0e8),
    stripe: hex(0xe6e9ef),
    hover: hex(0xccd0da),
    border: hex(0xacb0be),
    text: hex(0x4c4f69),
    strong: hex(0x11111b),
    weak: hex(0x5c5f77),
    accent: hex(0xc94f0c),
    on_accent: hex(0xffffff),
    label: hex(0x1a7f8a),
    green: hex(0x2f7d1f),
    red: hex(0xd20f39),
    yellow: hex(0xa86a10),
    blue: hex(0x1e66f5),
    magenta: hex(0x8839ef),
    dim: hex(0x8c8fa1),
    hit_bg: hex(0xf5d77a),
    hit_fg: hex(0x11111b),
};

/// A palette family with a dark and a light variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Family {
    /// drinsider's own high-contrast palettes.
    #[default]
    Default,
    /// Gruvbox.
    Gruvbox,
    /// Solarized.
    Solarized,
    /// Catppuccin (Mocha and Latte).
    Catppuccin,
}

impl Family {
    /// All families, in menu order.
    pub const ALL: [Family; 4] = [
        Family::Default,
        Family::Gruvbox,
        Family::Solarized,
        Family::Catppuccin,
    ];

    /// Name in the menu and the settings.
    pub fn name(self) -> &'static str {
        match self {
            Family::Default => "Default",
            Family::Gruvbox => "Gruvbox",
            Family::Solarized => "Solarized",
            Family::Catppuccin => "Catppuccin",
        }
    }

    /// The dark or light variant.
    pub fn palette(self, dark: bool) -> &'static Palette {
        match (self, dark) {
            (Family::Default, true) => &DARK,
            (Family::Default, false) => &LIGHT,
            (Family::Gruvbox, true) => &GRUVBOX_DARK,
            (Family::Gruvbox, false) => &GRUVBOX_LIGHT,
            (Family::Solarized, true) => &SOLARIZED_DARK,
            (Family::Solarized, false) => &SOLARIZED_LIGHT,
            (Family::Catppuccin, true) => &CATPPUCCIN_DARK,
            (Family::Catppuccin, false) => &CATPPUCCIN_LIGHT,
        }
    }
}

/// The theme setting: a palette family, and whether to follow the system's
/// light/dark preference or force one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Palette family.
    pub family: Family,
    /// System, light or dark.
    pub mode: ThemePreference,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            family: Family::Default,
            mode: ThemePreference::System,
        }
    }
}

impl Settings {
    /// Stored form, e.g. `gruvbox/system`.
    pub fn to_stored(self) -> String {
        let mode = match self.mode {
            ThemePreference::System => "system",
            ThemePreference::Light => "light",
            ThemePreference::Dark => "dark",
        };
        format!("{}/{mode}", self.family.name().to_lowercase())
    }

    /// Parses the stored form; anything unknown falls back to the default.
    pub fn from_stored(stored: &str) -> Self {
        let (family, mode) = stored.split_once('/').unwrap_or((stored, ""));
        Self {
            family: Family::ALL
                .into_iter()
                .find(|f| f.name().eq_ignore_ascii_case(family))
                .unwrap_or_default(),
            mode: match mode {
                "light" => ThemePreference::Light,
                "dark" => ThemePreference::Dark,
                _ => ThemePreference::System,
            },
        }
    }
}

fn family_id() -> egui::Id {
    egui::Id::new("theme_settings")
}

/// The palette of the active theme.
pub fn of(ui: &Ui) -> &'static Palette {
    current(ui.ctx()).family.palette(ui.visuals().dark_mode)
}

/// The active theme setting.
pub fn current(ctx: &egui::Context) -> Settings {
    ctx.data(|d| d.get_temp(family_id())).unwrap_or_default()
}

/// Installs a theme setting: both variants of its family, and the mode.
pub fn install(ctx: &egui::Context, settings: Settings) {
    let family = settings.family;
    ctx.style_mut_of(Theme::Dark, |style| apply(style, family.palette(true)));
    ctx.style_mut_of(Theme::Light, |style| apply(style, family.palette(false)));
    ctx.set_theme(settings.mode);
    ctx.data_mut(|d| d.insert_temp(family_id(), settings));
}

fn apply(style: &mut Style, p: &Palette) {
    use egui::FontFamily::{Monospace, Proportional};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(17.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        (TextStyle::Small, FontId::new(12.5, Proportional)),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(6.0, 2.0);
    spacing.button_padding = egui::vec2(10.0, 3.0);
    spacing.window_margin = egui::Margin::ZERO;
    spacing.menu_margin = egui::Margin::ZERO;
    spacing.interact_size.y = 22.0;

    let v = &mut style.visuals;
    v.override_text_color = None;
    v.weak_text_color = Some(p.weak);
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.accent);
    v.extreme_bg_color = p.bg;
    v.faint_bg_color = p.stripe;
    v.code_bg_color = p.hover;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.yellow;
    v.error_fg_color = p.red;
    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.0, p.on_accent);
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;

    let w = &mut v.widgets;
    for (widget, bg, fg, stroke) in [
        (&mut w.noninteractive, p.bg, p.text, p.border),
        (
            &mut w.inactive,
            Color32::TRANSPARENT,
            p.text,
            Color32::TRANSPARENT,
        ),
        (&mut w.hovered, p.hover, p.strong, p.accent),
        (&mut w.active, p.accent, p.on_accent, p.accent),
        (&mut w.open, p.hover, p.strong, p.accent),
    ] {
        widget.bg_fill = bg;
        widget.weak_bg_fill = bg;
        widget.fg_stroke = Stroke::new(1.0, fg);
        widget.bg_stroke = Stroke::new(1.0, stroke);
        widget.corner_radius = CornerRadius::ZERO;
        widget.expansion = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        for family in Family::ALL {
            for mode in [
                ThemePreference::System,
                ThemePreference::Light,
                ThemePreference::Dark,
            ] {
                let settings = Settings { family, mode };
                assert_eq!(Settings::from_stored(&settings.to_stored()), settings);
            }
        }
        assert_eq!(
            Settings::from_stored("gruvbox/dark").family,
            Family::Gruvbox
        );
        assert_eq!(Settings::from_stored("nonsense"), Settings::default());
    }
}
