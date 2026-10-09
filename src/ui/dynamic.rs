//! Dynamic tab: system calls of a traced run, in lurk's colors.

use super::{centered_message, flat_panel, row_height, section_bar, styled, theme};
use crate::ansi::Tone;
use crate::state::{DynamicTab, Trace};
use egui::{Color32, RichText, Ui};

/// Renders the dynamic analysis tab.
pub fn render(view: &mut DynamicTab, search: &str, ui: &mut Ui) {
    let p = theme::of(ui);
    let data = match &view.trace {
        Trace::Idle => {
            let message = if cfg!(target_arch = "wasm32") {
                "Dynamic analysis runs the binary under ptrace.\nIt is only available in the desktop app."
            } else {
                "Press Enter to run the executable."
            };
            return centered_message(ui, RichText::new(message).color(p.text));
        }
        Trace::Running => {
            egui::CentralPanel::default()
                .frame(flat_panel(ui))
                .show(ui, |ui| {
                    section_bar(ui, "System Calls", "running…", false);
                    ui.centered_and_justified(|ui| ui.spinner());
                });
            return;
        }
        Trace::Done(data) => data,
    };
    let tone = |tone: Tone| -> Color32 {
        match tone {
            Tone::Plain => p.text,
            Tone::Strong => p.strong,
            Tone::Red => p.red,
            Tone::Green => p.green,
            Tone::Yellow => p.yellow,
            Tone::Blue => p.blue,
            Tone::Magenta => p.magenta,
            Tone::Cyan => p.label,
        }
    };
    let scroll_to = view
        .scroll
        .take_moved(view.lines.len())
        .then_some(view.scroll.index);
    egui::CentralPanel::default()
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            section_bar(
                ui,
                "System Calls",
                &format!("Total: {}", view.lines.len()),
                false,
            );
            let row_height = row_height(ui);
            let mut area = egui::ScrollArea::both()
                .id_salt("syscalls")
                .auto_shrink(false);
            if let Some(index) = scroll_to {
                area = area.vertical_scroll_offset(
                    index as f32 * (row_height + ui.spacing().item_spacing.y),
                );
            }
            area.show_rows(ui, row_height, view.lines.len(), |ui, range| {
                for i in range {
                    let line = &data.syscalls[view.lines[i]];
                    let spans: Vec<_> = line
                        .spans
                        .iter()
                        .map(|(r, t)| (r.clone(), tone(*t)))
                        .collect();
                    let job = styled(ui, &format!(" {}", line.text), &shift(&spans, 1), search);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| ui.add(egui::Label::new(job).extend()),
                    );
                }
            });
        });
}

/// Moves byte ranges right by `by` (for a leading pad).
fn shift(
    spans: &[(std::ops::Range<usize>, Color32)],
    by: usize,
) -> Vec<(std::ops::Range<usize>, Color32)> {
    spans
        .iter()
        .map(|(r, c)| (r.start + by..r.end + by, *c))
        .collect()
}
