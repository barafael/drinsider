//! Static tab: file headers, notes, and the ELF tables with sub-tabs.

use super::{
    Action, GAP, copyable, flat_panel, highlighted, on_selection, row_height, section_bar,
    sort_header, tab_strip, theme,
};
use crate::analyzer::Analyzer;
use crate::command::Command;
use crate::elf::{INFO_TABS, Property};
use crate::list::Selection;
use crate::state::{Block, StaticTab};
use egui::{Align, Layout, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

/// Renders the static analysis tab; returns what the user clicked, if anything.
pub fn render(
    analyzer: &Analyzer,
    selection: &mut Selection,
    view: &mut StaticTab,
    search: &str,
    ui: &mut Ui,
) -> Option<Action> {
    let p = theme::of(ui);
    let headers = analyzer.elf.file_headers.items();
    let notes = analyzer.elf.notes.rows();
    let headers_scroll = view
        .headers_scroll
        .take_moved(headers.len())
        .then_some(view.headers_scroll.index);
    let notes_scroll = view
        .notes_scroll
        .take_moved(notes.len())
        .then_some(view.notes_scroll.index);
    let focused = view.block;

    // binsider splits the screen in half: headers and notes above, the table below.
    egui::Panel::top("static_top")
        .exact_size(ui.available_height() * 0.5)
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                section_bar(ui, "File Headers", "", focused == Block::Headers);
                egui::ScrollArea::vertical()
                    .id_salt("file_headers")
                    .show(ui, |ui| {
                        egui::Grid::new("file_headers_grid")
                            .num_columns(2)
                            .striped(true)
                            .min_row_height(row_height(ui))
                            .spacing([GAP * 2.0, 0.0])
                            .show(ui, |ui| {
                                for (i, row) in headers.iter().enumerate() {
                                    let response = ui.label(
                                        RichText::new(format!("  {}", row[0])).color(p.label),
                                    );
                                    copyable(
                                        ui,
                                        egui::Label::new(
                                            RichText::new(&row[1]).monospace().color(p.strong),
                                        ),
                                        &row[1],
                                    );
                                    ui.end_row();
                                    if headers_scroll == Some(i) {
                                        response.scroll_to_me(Some(Align::TOP));
                                    }
                                }
                            });
                    });

                let ui = &mut columns[1];
                section_bar(ui, "Notes", "", focused == Block::Notes);
                egui::ScrollArea::vertical()
                    .id_salt("notes")
                    .show(ui, |ui| {
                        if notes.is_empty() {
                            ui.label(RichText::new("  No notes.").color(p.weak).italics());
                        }
                        egui::Grid::new("notes_grid")
                            .num_columns(2)
                            .striped(true)
                            .min_row_height(row_height(ui))
                            .spacing([GAP * 2.0, 0.0])
                            .show(ui, |ui| {
                                for (i, (label, value)) in notes.iter().enumerate() {
                                    let response = if value.is_empty() {
                                        ui.label(
                                            RichText::new(format!("  {label}"))
                                                .italics()
                                                .color(p.accent),
                                        )
                                    } else {
                                        ui.label(RichText::new(format!("  {label}")).color(p.label))
                                    };
                                    if !value.is_empty() {
                                        copyable(
                                            ui,
                                            egui::Label::new(
                                                RichText::new(value).monospace().color(p.strong),
                                            )
                                            .truncate(),
                                            value,
                                        );
                                    } else {
                                        ui.label("");
                                    }
                                    ui.end_row();
                                    if notes_scroll == Some(i) {
                                        response.scroll_to_me(Some(Align::TOP));
                                    }
                                }
                            });
                    });
            });
        });

    let selected = selection.selected;
    let scroll_to = selection.take_moved().then_some(selected).flatten();
    let info = view.info();
    let columns = info.headers();
    let rows = &view.rows;
    let sort = view.sort;
    let mut action = None;
    egui::CentralPanel::default()
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            let titles: Vec<&str> = INFO_TABS.iter().map(|t| t.title()).collect();
            let counter = match selected {
                Some(s) => format!("{}/{}", s + 1, rows.len()),
                None => format!("{} rows", rows.len()),
            };
            action = tab_strip(
                ui,
                &titles,
                view.info_index,
                focused == Block::Table,
                &counter,
            )
            .map(Action::SelectInfo);
            if rows.is_empty() {
                ui.label(RichText::new("  Nothing to show.").color(p.weak).italics());
                return;
            }
            let row_height = row_height(ui);
            // Evenly split columns over the full width, like binsider's table.
            let mut table = TableBuilder::new(ui)
                .id_salt(("elf_table", view.info_index))
                .striped(true)
                .sense(Sense::CLICK)
                .cell_layout(Layout::left_to_right(Align::Center))
                .columns(Column::remainder().clip(true), columns.len())
                .max_scroll_height(f32::INFINITY);
            if let Some(row) = scroll_to {
                table = table.scroll_to_row(row, None);
            }
            table
                .header(row_height, |mut header| {
                    for (i, column) in columns.iter().enumerate() {
                        header.col(|ui| {
                            if sort_header(ui, &format!("  {column}"), sort, i) {
                                action = Some(Action::SortStatic(i));
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_height, rows.len(), |mut row| {
                        let i = row.index();
                        let is_selected = selected == Some(i);
                        row.set_selected(is_selected);
                        for cell in &rows[i] {
                            row.col(|ui| {
                                if is_selected {
                                    on_selection(ui);
                                }
                                // Truncated cells show their full text on hover.
                                ui.add(
                                    egui::Label::new(highlighted(ui, &format!("  {cell}"), search))
                                        .truncate(),
                                );
                            });
                        }
                        let response = row.response();
                        if response.clicked() {
                            selection.select(i);
                        }
                        if response.double_clicked() {
                            action = Some(Action::Run(Command::ShowDetails));
                        }
                    });
                });
        });
    action
}
