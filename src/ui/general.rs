//! General tab: file information and dependencies.

use super::{Action, GAP, copyable, flat_panel, on_selection, row_height, section_bar, theme};
use crate::analyzer::Analyzer;
use crate::command::Command;
use crate::list::Selection;
use crate::state::GeneralTab;
use egui::{Align, Label, Layout, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

/// Renders the general tab; returns what the user clicked, if anything.
pub fn render(
    analyzer: &Analyzer,
    selection: &mut Selection,
    view: &mut GeneralTab,
    ui: &mut Ui,
) -> Option<Action> {
    let mut action = None;
    let p = theme::of(ui);
    let file = &analyzer.file;
    let lines = file.rows();
    let scroll_to = view
        .info_scroll
        .take_moved(lines.len())
        .then_some(view.info_scroll.index);

    egui::Panel::left("file_info")
        .resizable(true)
        .default_size(460.0)
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            section_bar(ui, &format!("File: {}", file.name), "", false);
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(GAP as i8, 4))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(analyzer.path.to_string_lossy())
                            .monospace()
                            .color(p.weak),
                    );
                });
            egui::ScrollArea::vertical()
                .id_salt("file_info_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("file_info_grid")
                        .num_columns(2)
                        .striped(true)
                        .min_row_height(row_height(ui))
                        .spacing([GAP * 2.0, 0.0])
                        .show(ui, |ui| {
                            for (i, (label, value)) in lines.iter().enumerate() {
                                let response =
                                    ui.label(RichText::new(format!("  {label}")).color(p.label));
                                copyable(
                                    ui,
                                    Label::new(RichText::new(*value).monospace().color(p.strong)),
                                    value,
                                );
                                ui.end_row();
                                // h/l move the panel by whole rows; the mouse wheel still works in between.
                                if scroll_to == Some(i) {
                                    response.scroll_to_me(Some(Align::TOP));
                                }
                            }
                        });
                });
        });

    let dependencies = &analyzer.dependencies;
    let selected = selection.selected;
    let scroll_to_selected = selection.take_moved().then_some(selected).flatten();
    egui::CentralPanel::default()
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            let counter = match selected {
                Some(s) => format!("{}/{}", s + 1, dependencies.len()),
                None => format!("{} libraries", dependencies.len()),
            };
            section_bar(ui, "Dependencies", &counter, false);
            if dependencies.is_empty() {
                ui.label(
                    RichText::new("  No shared library dependencies.")
                        .color(p.weak)
                        .italics(),
                );
                return;
            }
            let row_height = row_height(ui);
            let mut table = TableBuilder::new(ui)
                .id_salt("dependencies")
                .striped(true)
                .sense(Sense::CLICK)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::auto().at_least(160.0))
                .column(Column::remainder().clip(true))
                .max_scroll_height(f32::INFINITY);
            if let Some(row) = scroll_to_selected {
                table = table.scroll_to_row(row, None);
            }
            table
                .header(row_height, |mut header| {
                    header.col(|ui| {
                        ui.label(RichText::new("  Library").strong().color(p.label));
                    });
                    header.col(|ui| {
                        ui.label(RichText::new("Path").strong().color(p.label));
                    });
                })
                .body(|body| {
                    body.rows(row_height, dependencies.len(), |mut row| {
                        let i = row.index();
                        let dependency = &dependencies[i];
                        let is_selected = selected == Some(i);
                        row.set_selected(is_selected);
                        row.col(|ui| {
                            if is_selected {
                                on_selection(ui);
                            }
                            ui.label(RichText::new(format!("  {}", dependency.name)).monospace());
                        });
                        row.col(|ui| {
                            if is_selected {
                                on_selection(ui);
                            }
                            match &dependency.path {
                                Some(path) => ui.add(
                                    Label::new(RichText::new(path.to_string_lossy()).monospace())
                                        .truncate(),
                                ),
                                None if analyzer.resolved => {
                                    let text = RichText::new("not found").italics();
                                    ui.label(if is_selected { text } else { text.color(p.red) })
                                }
                                None => ui.label(
                                    RichText::new("not resolved in the browser")
                                        .italics()
                                        .color(if is_selected { p.on_accent } else { p.weak }),
                                ),
                            };
                        });
                        let response = row.response();
                        if response.clicked() {
                            selection.select(i);
                        }
                        // Like Enter: analyze the library.
                        if response.double_clicked() {
                            action = Some(Action::Run(Command::ShowDetails));
                        }
                    });
                });
        });
    action
}
