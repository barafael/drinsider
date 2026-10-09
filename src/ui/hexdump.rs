//! Hexdump tab: heh's layout (address, hex and ASCII panes with a grid of
//! decoded values below) as a read-only viewer.

use super::{GAP, char_width, flat_panel, row_height, section_bar, theme};
use crate::analyzer::Analyzer;
use crate::state::HexdumpTab;
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, Layout, RichText, Sense, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use theme::Palette;

/// heh's ASCII rendering of a byte: escape character and color category.
fn glyph(byte: u8, p: &Palette) -> (char, Color32) {
    match byte {
        0 => ('0', p.dim),
        b' ' => (' ', p.green),
        b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => ('_', p.green),
        0x01..=0x1f | 0x7f => ('•', p.magenta),
        0x21..=0x7e => (byte as char, p.label),
        // heh prints U+FFFD, which egui's bundled fonts lack.
        _ => ('♦', p.yellow),
    }
}

/// Renders the hexdump tab.
pub fn render(analyzer: &Analyzer, view: &mut HexdumpTab, notice: Option<&str>, ui: &mut Ui) {
    let p = theme::of(ui);
    let bytes = &analyzer.bytes;
    let moved = view.cursor.take_moved(bytes.len());
    let cursor = view.cursor.index;

    egui::Panel::bottom("hex_values")
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            render_values(bytes, cursor, view.big_endian, notice, ui)
        });

    egui::CentralPanel::default()
        .frame(flat_panel(ui))
        .show(ui, |ui| {
            section_bar(
                ui,
                "Hexdump",
                &format!("{cursor:#X} / {:#X}", bytes.len()),
                false,
            );
            // Fit as many bytes per row as the width allows: 3 characters of hex
            // plus 1 of ASCII per byte, after a 10-character address column.
            let cw = char_width(ui);
            let spare = ui.available_width() - 14.0 * cw - 4.0 * GAP;
            let columns = ((spare / (4.0 * cw)) as usize).max(1);
            view.columns = columns;
            let rows = bytes.len().div_ceil(columns);
            let font_id = TextStyle::Monospace.resolve(ui.style());
            let format = |color: Color32, background: Color32| TextFormat {
                font_id: font_id.clone(),
                color,
                background,
                ..Default::default()
            };
            let row_height = row_height(ui);
            let mut table = TableBuilder::new(ui)
                .id_salt("hexdump")
                .striped(true)
                .sense(Sense::CLICK)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(10.0 * cw + GAP))
                .column(Column::exact(3.0 * cw * columns as f32 + GAP))
                .column(Column::remainder())
                .max_scroll_height(f32::INFINITY);
            if moved {
                table = table.scroll_to_row(cursor / columns, None);
            }
            table
                .header(row_height, |mut header| {
                    for title in ["  Address", "Hex", "ASCII"] {
                        header.col(|ui| {
                            ui.label(RichText::new(title).strong().color(p.label));
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_height, rows, |mut row| {
                        let r = row.index();
                        let start = r * columns;
                        let chunk = &bytes[start..(start + columns).min(bytes.len())];
                        let on_row = cursor / columns == r;
                        row.col(|ui| {
                            let color = if on_row { p.accent } else { p.weak };
                            ui.label(
                                RichText::new(format!("  {start:08X}"))
                                    .monospace()
                                    .color(color),
                            );
                        });
                        let mut hex = LayoutJob::default();
                        let mut ascii = LayoutJob::default();
                        for (i, &byte) in chunk.iter().enumerate() {
                            let (escape, color) = glyph(byte, p);
                            let (fg, bg) = if on_row && start + i == cursor {
                                (p.on_accent, p.accent)
                            } else {
                                (color, Color32::TRANSPARENT)
                            };
                            hex.append(&format!("{byte:02X}"), 0.0, format(fg, bg));
                            hex.append(" ", 0.0, format(p.text, Color32::TRANSPARENT));
                            ascii.append(&escape.to_string(), 0.0, format(fg, bg));
                        }
                        row.col(|ui| {
                            ui.label(hex);
                        });
                        row.col(|ui| {
                            ui.label(ascii);
                        });
                        if row.response().clicked() {
                            view.cursor.jump(start);
                        }
                    });
                });
        });
}

/// heh's dashboard: the bytes at the cursor decoded several ways.
fn render_values(bytes: &[u8], cursor: usize, big_endian: bool, notice: Option<&str>, ui: &mut Ui) {
    let p = theme::of(ui);
    let endian = if big_endian {
        "big endian"
    } else {
        "little endian"
    };
    section_bar(ui, "Values", &format!("{endian}  (n to toggle)"), false);
    let at = |n: usize| -> Option<u64> {
        let slice = bytes.get(cursor..cursor + n)?;
        let mut buf = [0u8; 8];
        Some(if big_endian {
            buf[8 - n..].copy_from_slice(slice);
            u64::from_be_bytes(buf)
        } else {
            buf[..n].copy_from_slice(slice);
            u64::from_le_bytes(buf)
        })
    };
    let signed = |n: usize| at(n).map(|v| ((v << (64 - 8 * n)) as i64) >> (64 - 8 * n));
    let show = |v: Option<String>| v.unwrap_or_else(|| "–".to_string());
    let byte = bytes.get(cursor).copied();
    // Columns as in heh: integers, wider integers, bit views, floats/offset.
    let cells: [[(&str, String); 4]; 4] = [
        [
            ("Signed 8 bit", show(signed(1).map(|v| v.to_string()))),
            ("Unsigned 8 bit", show(at(1).map(|v| v.to_string()))),
            ("Signed 16 bit", show(signed(2).map(|v| v.to_string()))),
            ("Unsigned 16 bit", show(at(2).map(|v| v.to_string()))),
        ],
        [
            ("Signed 32 bit", show(signed(4).map(|v| v.to_string()))),
            ("Unsigned 32 bit", show(at(4).map(|v| v.to_string()))),
            ("Signed 64 bit", show(signed(8).map(|v| v.to_string()))),
            ("Unsigned 64 bit", show(at(8).map(|v| v.to_string()))),
        ],
        [
            ("Hexadecimal", show(byte.map(|b| format!("{b:02X}")))),
            ("Octal", show(byte.map(|b| format!("{b:03o}")))),
            ("Binary", show(byte.map(|b| format!("{b:08b}")))),
            ("Stream Length", String::from("8")),
        ],
        [
            (
                "Float 32 bit",
                show(at(4).map(|v| format!("{:e}", f32::from_bits(v as u32)))),
            ),
            (
                "Float 64 bit",
                show(at(8).map(|v| format!("{:e}", f64::from_bits(v)))),
            ),
            ("Offset", format!("{cursor:#X}")),
            ("Notifications", notice.unwrap_or_default().to_string()),
        ],
    ];
    ui.columns(4, |columns| {
        for (i, (ui, column)) in columns.iter_mut().zip(&cells).enumerate() {
            egui::Grid::new(("hex_values", i))
                .num_columns(2)
                .striped(true)
                .min_row_height(row_height(ui))
                .spacing([GAP, 0.0])
                .show(ui, |ui| {
                    for (label, value) in column {
                        ui.label(RichText::new(format!("  {label}")).color(p.label));
                        ui.add(
                            egui::Label::new(RichText::new(value).monospace().color(p.strong))
                                .truncate(),
                        );
                        ui.end_row();
                    }
                });
        }
    });
}
