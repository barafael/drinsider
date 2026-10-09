//! Window rendering: header with tab strip, status bar, and the per-tab body.

mod dynamic;
mod general;
mod hexdump;
mod static_analysis;
mod strings;
pub mod theme;

use crate::state::{Prompt, State, Tab};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Align2, Color32, Label, Layout, RichText, Sense, TextStyle, Ui};
use std::ops::Range;

pub use theme::install;

/// Horizontal gap between unrelated items.
const GAP: f32 = 10.0;

/// Panel frame without padding (the default panel frames add 8 px).
pub fn flat_panel(ui: &Ui) -> egui::Frame {
    egui::Frame::NONE.fill(ui.visuals().panel_fill)
}

/// Height of one table row.
pub fn row_height(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y
}

/// Width of one monospace character.
pub fn char_width(ui: &Ui) -> f32 {
    ui.painter()
        .layout_no_wrap(
            "0".into(),
            TextStyle::Monospace.resolve(ui.style()),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
}

/// A full-width bar titling a block; filled with the accent when the block
/// has keyboard focus, like binsider's highlighted borders.
pub fn section_bar(ui: &mut Ui, title: &str, right: &str, focused: bool) {
    let p = theme::of(ui);
    let height = ui.text_style_height(&TextStyle::Heading) + 8.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let (bg, fg, sub) = if focused {
        (p.accent, p.on_accent, p.on_accent)
    } else {
        (p.surface, p.strong, p.weak)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg);
    painter.text(
        rect.left_center() + egui::vec2(GAP, 0.0),
        Align2::LEFT_CENTER,
        title,
        TextStyle::Heading.resolve(ui.style()),
        fg,
    );
    painter.text(
        rect.right_center() - egui::vec2(GAP, 0.0),
        Align2::RIGHT_CENTER,
        right,
        TextStyle::Body.resolve(ui.style()),
        sub,
    );
}

/// A row of tabs on a bar; returns the clicked one. On a focused bar the
/// selected tab is drawn inverted so it still stands out.
pub fn tab_strip(
    ui: &mut Ui,
    titles: &[&str],
    selected: usize,
    focused: bool,
    right: &str,
) -> Option<usize> {
    let p = theme::of(ui);
    let bar = if focused { p.accent } else { p.surface };
    let mut clicked = None;
    egui::Frame::NONE.fill(bar).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, title) in titles.iter().enumerate() {
                let (bg, fg) = match (i == selected, focused) {
                    (true, false) => (p.accent, p.on_accent),
                    (true, true) => (p.bg, p.accent),
                    (false, true) => (Color32::TRANSPARENT, p.on_accent),
                    (false, false) => (Color32::TRANSPARENT, p.text),
                };
                let text = RichText::new(*title).color(fg);
                let text = if i == selected { text.strong() } else { text };
                // `Sense::CLICK` alone is not focusable, so Tab keeps reaching the keymap.
                let response = egui::Frame::NONE
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(GAP as i8, 4))
                    .show(ui, |ui| {
                        ui.add(Label::new(text).sense(Sense::CLICK).selectable(false))
                    })
                    .inner;
                if response.clicked() {
                    clicked = Some(i);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(GAP);
                ui.label(RichText::new(right).color(if focused { p.on_accent } else { p.weak }));
            });
        });
    });
    clicked
}

/// Monospace text with colored byte ranges and every case-insensitive
/// occurrence of `query` highlighted.
pub fn styled(ui: &Ui, text: &str, spans: &[(Range<usize>, Color32)], query: &str) -> LayoutJob {
    let p = theme::of(ui);
    let base = ui.visuals().text_color();
    let font_id = TextStyle::Monospace.resolve(ui.style());
    let mut hits: Vec<Range<usize>> = Vec::new();
    let lower = text.to_lowercase();
    // Byte offsets of `lower` match `text` only when lowercasing kept the length.
    if !query.is_empty() && lower.len() == text.len() {
        let query = query.to_lowercase();
        let mut from = 0;
        while let Some(i) = lower[from..].find(&query) {
            let start = from + i;
            let end = start + query.len();
            if text.is_char_boundary(start) && text.is_char_boundary(end) {
                hits.push(start..end);
            }
            from = end.max(start + 1);
        }
    }
    let mut cuts: Vec<usize> = vec![0, text.len()];
    for range in spans.iter().map(|(r, _)| r).chain(&hits) {
        cuts.extend([range.start, range.end]);
    }
    cuts.retain(|&c| c <= text.len() && text.is_char_boundary(c));
    cuts.sort_unstable();
    cuts.dedup();
    let mut job = LayoutJob::default();
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let hit = hits.iter().any(|h| h.start <= a && b <= h.end);
        let color = spans
            .iter()
            .find(|(r, _)| r.start <= a && b <= r.end)
            .map_or(base, |(_, c)| *c);
        job.append(
            &text[a..b],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: if hit { p.hit_fg } else { color },
                background: if hit { p.hit_bg } else { Color32::TRANSPARENT },
                ..Default::default()
            },
        );
    }
    job
}

/// Monospace text with search hits highlighted.
pub fn highlighted(ui: &Ui, text: &str, query: &str) -> LayoutJob {
    styled(ui, text, &[], query)
}

/// Makes text in a selected table row readable on the accent background.
pub fn on_selection(ui: &mut Ui) {
    let on_accent = theme::of(ui).on_accent;
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(on_accent);
    visuals.weak_text_color = Some(on_accent);
}

/// Renders the whole window.
pub fn render(state: &mut State, ui: &mut Ui) {
    egui::Panel::top("header")
        .frame(flat_panel(ui))
        .show_separator_line(false)
        .show(ui, |ui| render_header(state, ui));
    egui::Panel::bottom("status")
        .frame(flat_panel(ui))
        .show(ui, |ui| render_status(state, ui));
    let Some(analyzer) = state.files.last() else {
        let message = state.fatal.as_deref().unwrap_or_default();
        let hint = if cfg!(target_arch = "wasm32") {
            "\n\nDrop an ELF file here or press Ctrl+O."
        } else {
            ""
        };
        centered_message(
            ui,
            RichText::new(format!("{message}{hint}")).color(ui.visuals().error_fg_color),
        );
        return;
    };
    match state.tab {
        Tab::General => general::render(analyzer, &mut state.selection, &mut state.general, ui),
        Tab::StaticAnalysis => {
            let clicked = static_analysis::render(
                analyzer,
                &mut state.selection,
                &mut state.static_,
                &state.search,
                ui,
            );
            if let Some(index) = clicked {
                state.select_info(index);
            }
        }
        Tab::DynamicAnalysis => dynamic::render(&mut state.dynamic, &state.search, ui),
        Tab::Strings => {
            strings::render(&mut state.selection, &mut state.strings, &state.search, ui)
        }
        Tab::Hexdump => hexdump::render(analyzer, &mut state.hexdump, state.notice.as_deref(), ui),
    }
    if state.show_details {
        render_details(state, ui);
    }
}

/// Fills the body with one centered message.
pub fn centered_message(ui: &mut Ui, text: RichText) {
    egui::CentralPanel::default()
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            ui.centered_and_justified(|ui| ui.label(text));
        });
}

/// The details popup: the selected row as label/value pairs, or the trace summary.
fn render_details(state: &State, ui: &mut Ui) {
    let title = match state.tab {
        Tab::DynamicAnalysis => "Summary",
        _ => "Details",
    };
    let rows: Vec<(String, String)> = match state.tab {
        Tab::StaticAnalysis => state
            .selected_row()
            .map(|row| {
                let headers = state.static_.info().headers();
                headers
                    .iter()
                    .zip(row)
                    .map(|(h, v)| (h.to_string(), v.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        Tab::Strings => state
            .selected_string()
            .map(|(offset, s)| {
                vec![
                    ("Location".into(), format!("{offset} ({offset:#x})")),
                    ("String".into(), s.clone()),
                ]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let summary = match state.tab {
        Tab::DynamicAnalysis => state.dynamic.data().map(|d| d.summary.clone()),
        _ => None,
    };
    let p = theme::of(ui);
    let frame = egui::Frame::popup(ui.style())
        .corner_radius(0.0)
        .inner_margin(0.0)
        .stroke(egui::Stroke::new(1.0, p.accent));
    egui::Modal::new(egui::Id::new("details"))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(160))
        .show(ui.ctx(), |ui| {
            let screen = ui.ctx().content_rect();
            ui.set_max_width(screen.width() * 0.8);
            section_bar(ui, title, "Esc or Enter to close", true);
            egui::Frame::NONE.inner_margin(GAP).show(ui, |ui| {
                if let Some(summary) = summary {
                    egui::ScrollArea::both()
                        .max_height(screen.height() * 0.7)
                        .show(ui, |ui| {
                            ui.monospace(summary);
                        });
                } else {
                    egui::Grid::new("details_grid")
                        .num_columns(2)
                        .spacing([GAP * 2.0, 4.0])
                        .show(ui, |ui| {
                            for (label, value) in &rows {
                                ui.label(RichText::new(label).color(p.label));
                                ui.add(
                                    Label::new(RichText::new(value).monospace().color(p.strong))
                                        .wrap(),
                                );
                                ui.end_row();
                            }
                        });
                }
            });
        });
}

fn render_header(state: &mut State, ui: &mut Ui) {
    let p = theme::of(ui);
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(GAP as i8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("drinsider")
                        .heading()
                        .strong()
                        .color(p.accent),
                );
                ui.weak(env!("CARGO_PKG_VERSION"));
                ui.add_space(GAP);
                ui.label(
                    RichText::new("Analyze ELF binaries like a normal person.")
                        .italics()
                        .color(p.weak),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Breadcrumb of analyzed files, current one last.
                    let mut files = state.files.iter().rev();
                    if let Some(current) = files.next() {
                        ui.monospace(RichText::new(current.path.to_string_lossy()).color(p.strong));
                    }
                    for earlier in files {
                        ui.label(RichText::new("›").color(p.weak));
                        ui.monospace(RichText::new(&earlier.file.name).color(p.weak));
                    }
                });
            });
        });
    let titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    if let Some(i) = tab_strip(ui, &titles, state.tab as usize, false, "") {
        state.select_tab(Tab::ALL[i]);
    }
}

fn render_status(state: &State, ui: &mut Ui) {
    let p = theme::of(ui);
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(GAP as i8, 4))
        .show(ui, |ui| {
            if let Some(notice) = &state.notice {
                ui.label(RichText::new(notice).color(p.red).strong());
            }
            ui.horizontal_wrapped(|ui| {
                // The open prompt, or the confirmed query that still filters the list.
                let prompt = match state.prompt {
                    Some(Prompt::Search) => Some(("search", state.search.as_str(), true)),
                    Some(Prompt::Jump) => Some(("jump to", state.jump.as_str(), true)),
                    None if !state.search.is_empty() => {
                        Some(("search", state.search.as_str(), false))
                    }
                    None => None,
                };
                if let Some((label, text, open)) = prompt {
                    ui.label(RichText::new(format!("{label}:")).color(p.label));
                    let text = RichText::new(format!(" {text}{} ", if open { "▏" } else { "" }))
                        .monospace();
                    ui.label(if open {
                        text.color(p.on_accent).background_color(p.accent)
                    } else {
                        text.color(p.strong)
                    });
                    ui.add_space(GAP * 2.0);
                }
                for (keys, desc) in state.key_bindings() {
                    ui.label(
                        RichText::new(format!(" {keys} "))
                            .monospace()
                            .color(p.accent)
                            .background_color(p.hover),
                    );
                    ui.label(RichText::new(desc).color(p.text));
                    ui.add_space(GAP);
                }
            });
        });
}
