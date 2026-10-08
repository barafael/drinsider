//! Strings tab: printable sequences found in the file.

use super::{flat_panel, highlighted, on_selection, row_height, section_bar, theme};
use crate::list::Selection;
use crate::state::StringsTab;
use egui::{Align, Layout, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

/// Renders the strings tab.
pub fn render(selection: &mut Selection, view: &mut StringsTab, search: &str, ui: &mut Ui) {
    let p = theme::of(ui);
    let selected = selection.selected;
    let scroll_to = selection.take_moved().then_some(selected).flatten();
    egui::CentralPanel::default().frame(flat_panel(ui)).show(ui, |ui| {
        let counter = match selected {
            Some(s) => format!("{}/{}", s + 1, view.rows.len()),
            None => format!("{} strings", view.rows.len()),
        };
        section_bar(ui, "Strings", &format!("Min length: {}    {counter}", view.min_length), false);
        let Some(found) = &view.found else {
            ui.horizontal(|ui| {
                ui.add_space(super::GAP);
                ui.spinner();
                ui.label(RichText::new("Loading…").italics().color(p.weak));
            });
            return;
        };
        if view.rows.is_empty() {
            ui.label(RichText::new("  No strings found.").color(p.weak).italics());
            return;
        }
        let row_height = row_height(ui);
        // binsider prints decimal offsets, right-aligned.
        let digits = view.rows.iter().map(|&i| found[i].0).max().unwrap_or(0).to_string().len();
        let mut table = TableBuilder::new(ui)
            .id_salt("strings")
            .striped(true)
            .sense(Sense::CLICK)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto().at_least(80.0))
            .column(Column::remainder().clip(true))
            .max_scroll_height(f32::INFINITY);
        if let Some(row) = scroll_to {
            table = table.scroll_to_row(row, None);
        }
        table
            .header(row_height, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("  Location").strong().color(p.label));
                });
                header.col(|ui| {
                    ui.label(RichText::new("String").strong().color(p.label));
                });
            })
            .body(|body| {
                body.rows(row_height, view.rows.len(), |mut row| {
                    let i = row.index();
                    let (offset, text) = &found[view.rows[i]];
                    let is_selected = selected == Some(i);
                    row.set_selected(is_selected);
                    row.col(|ui| {
                        let color = if is_selected { p.on_accent } else { p.label };
                        ui.label(RichText::new(format!("  {offset:>digits$}")).monospace().color(color));
                    });
                    row.col(|ui| {
                        if is_selected {
                            on_selection(ui);
                        }
                        let single_line: String = text.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).collect();
                        ui.label(highlighted(ui, &single_line, search));
                    });
                    if row.response().clicked() {
                        selection.select(i);
                    }
                });
            });
    });
}
