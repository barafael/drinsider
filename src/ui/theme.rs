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
pub const DARK: Palette = Palette {
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
pub const LIGHT: Palette = Palette {
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

/// The palette of the active theme.
pub fn of(ui: &Ui) -> &'static Palette {
    if ui.visuals().dark_mode {
        &DARK
    } else {
        &LIGHT
    }
}

/// Installs both themes; the system preference picks one.
pub fn install(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Dark, |style| apply(style, &DARK));
    ctx.style_mut_of(Theme::Light, |style| apply(style, &LIGHT));
    ctx.set_theme(ThemePreference::System);
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
