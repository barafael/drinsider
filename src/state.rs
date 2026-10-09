//! Application state and command handling.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

use crate::analyzer::{Analyzer, Dependency};
use crate::command::{Command, InputCommand, ScrollType};
use crate::elf::{INFO_TABS, Info, Property};
use crate::list::{Scroll, Selection};
use crate::strings::{Extraction, Found};
use crate::tracer::{TraceData, TraceOptions, Traced};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// Documentation URL opened with `o`.
pub const DOCS_URL: &str = "https://binsider.dev";

/// Default minimum length of extracted strings (binsider's `--min-len`).
pub const DEFAULT_MIN_STRING_LEN: usize = 15;

/// Bytes per hexdump row until the renderer knows the window width.
pub const HEX_COLS: usize = 16;

/// Startup options (binsider's command line flags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Minimum length of extracted strings.
    pub min_strings_len: usize,
    /// Tab shown first.
    pub tab: Tab,
    /// Arguments passed to the binary when tracing it.
    pub arguments: Vec<String>,
    /// lurk formatting options.
    pub trace: TraceOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            min_strings_len: DEFAULT_MIN_STRING_LEN,
            tab: Tab::General,
            arguments: Vec::new(),
            trace: TraceOptions::default(),
        }
    }
}

/// Application tab.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Tab {
    /// General information.
    #[default]
    General,
    /// Static analysis.
    StaticAnalysis,
    /// Dynamic analysis.
    DynamicAnalysis,
    /// Strings.
    Strings,
    /// Hexdump.
    Hexdump,
}

impl Tab {
    /// All tabs in display order.
    pub const ALL: [Tab; 5] = [
        Tab::General,
        Tab::StaticAnalysis,
        Tab::DynamicAnalysis,
        Tab::Strings,
        Tab::Hexdump,
    ];

    /// Tab title.
    pub fn title(self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::StaticAnalysis => "Static",
            Tab::DynamicAnalysis => "Dynamic",
            Tab::Strings => "Strings",
            Tab::Hexdump => "Hexdump",
        }
    }

    /// The tab `amount` positions to the right, wrapping.
    pub fn next(self, amount: usize) -> Tab {
        Self::ALL[(self as usize + amount) % Self::ALL.len()]
    }

    /// The tab `amount` positions to the left, wrapping to the last.
    pub fn previous(self, amount: usize) -> Tab {
        Self::ALL[(self as usize)
            .checked_sub(amount)
            .unwrap_or(Self::ALL.len() - 1)]
    }

    /// Parses binsider's `--tab` values (plus the short titles).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn from_name(name: &str) -> Option<Tab> {
        match name.to_ascii_lowercase().as_str() {
            "general" => Some(Tab::General),
            "static" | "static-analysis" => Some(Tab::StaticAnalysis),
            "dynamic" | "dynamic-analysis" => Some(Tab::DynamicAnalysis),
            "strings" => Some(Tab::Strings),
            "hexdump" => Some(Tab::Hexdump),
            _ => None,
        }
    }
}

/// Which prompt is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompt {
    /// Filter the current list.
    Search,
    /// Jump to a byte offset (hexdump).
    Jump,
}

/// View state of the General tab.
#[derive(Debug, Default)]
pub struct GeneralTab {
    /// Scroll offset of the file info panel.
    pub info_scroll: Scroll,
}

/// Focused block of the static analysis tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Block {
    /// File headers.
    Headers,
    /// Notes.
    Notes,
    /// ELF table.
    #[default]
    Table,
}

impl Block {
    const ALL: [Block; 3] = [Block::Headers, Block::Notes, Block::Table];

    fn next(self) -> Block {
        Self::ALL[(self as usize + 1) % 3]
    }

    fn previous(self) -> Block {
        Self::ALL[(self as usize + 2) % 3]
    }
}

/// View state of the Static tab.
#[derive(Debug, Default)]
pub struct StaticTab {
    /// Index into [`INFO_TABS`].
    pub info_index: usize,
    /// Focused block.
    pub block: Block,
    /// Scroll offset of the file headers block.
    pub headers_scroll: Scroll,
    /// Scroll offset of the notes block.
    pub notes_scroll: Scroll,
    /// Rows of the current table after filtering and sorting.
    pub rows: Vec<Vec<String>>,
    /// Column the table is sorted by, if any.
    pub sort: Option<Sort>,
}

/// A table sorted by one column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sort {
    /// Column index.
    pub column: usize,
    /// Largest first.
    pub descending: bool,
}

impl Sort {
    /// Clicking a header: ascending, then descending, then unsorted.
    fn cycle(sort: Option<Sort>, column: usize) -> Option<Sort> {
        match sort {
            Some(Sort {
                column: c,
                descending: false,
            }) if c == column => Some(Sort {
                column,
                descending: true,
            }),
            Some(Sort { column: c, .. }) if c == column => None,
            _ => Some(Sort {
                column,
                descending: false,
            }),
        }
    }

    /// Orders `a` and `b` in this direction.
    fn apply(self, ordering: std::cmp::Ordering) -> std::cmp::Ordering {
        if self.descending {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

/// Compares table cells: by value if both are numbers (decimal, `0x` hex or
/// a readable size such as `1.5 KiB`), numbers before text, otherwise as
/// text ignoring case.
fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    fn number(cell: &str) -> Option<f64> {
        let cell = cell.trim();
        if let Some(hex) = cell.strip_prefix("0x") {
            return u64::from_str_radix(hex, 16).ok().map(|n| n as f64);
        }
        cell.parse::<f64>().ok().or_else(|| {
            cell.parse::<bytesize::ByteSize>()
                .ok()
                .map(|s| s.as_u64() as f64)
        })
    }
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

impl StaticTab {
    /// The current ELF table.
    pub fn info(&self) -> Info {
        INFO_TABS[self.info_index]
    }
}

/// The search query as a row filter.
struct Filter(String);

impl Filter {
    fn new(query: &str) -> Self {
        Self(query.to_lowercase())
    }

    /// Whether any cell contains the query, case-insensitively.
    fn matches<'a>(&self, cells: impl IntoIterator<Item = &'a str>) -> bool {
        self.0.is_empty() || cells.into_iter().any(|cell| self.found_in(cell))
    }

    fn found_in(&self, cell: &str) -> bool {
        // ASCII cells (nearly all of them) are compared in place instead of
        // lowercasing a copy of each one.
        if cell.is_ascii() {
            cell.as_bytes()
                .windows(self.0.len())
                .any(|w| w.eq_ignore_ascii_case(self.0.as_bytes()))
        } else {
            cell.to_lowercase().contains(&self.0)
        }
    }
}

/// View state of the Strings tab.
#[derive(Debug, Default)]
pub struct StringsTab {
    /// Minimum string length.
    pub min_length: usize,
    /// Extracted strings; `None` while extracting.
    pub found: Option<Vec<Found>>,
    /// Indices into `found` after filtering and sorting.
    pub rows: Vec<usize>,
    /// Column the list is sorted by; unsorted is by location.
    pub sort: Option<Sort>,
    /// Digits of the widest location shown, to right-align them.
    pub offset_digits: usize,
    /// Extraction in progress.
    pub pending: Option<Extraction>,
}

/// Progress of the dynamic analysis.
#[derive(Debug, Default)]
pub enum Trace {
    /// Not run yet.
    #[default]
    Idle,
    /// Running under ptrace.
    Running,
    /// Finished.
    Done(TraceData),
}

/// View state of the Dynamic tab.
#[derive(Debug, Default)]
pub struct DynamicTab {
    /// Trace progress.
    pub trace: Trace,
    /// Indices into the trace lines after filtering.
    pub lines: Vec<usize>,
    /// Scroll offset of the system call list.
    pub scroll: Scroll,
    /// Trace in progress.
    pending: Option<Receiver<Traced>>,
}

impl DynamicTab {
    /// The finished trace, if any.
    pub fn data(&self) -> Option<&TraceData> {
        match &self.trace {
            Trace::Done(data) => Some(data),
            _ => None,
        }
    }
}

/// View state of the Hexdump tab.
#[derive(Debug)]
pub struct HexdumpTab {
    /// Byte offset under the cursor.
    pub cursor: Scroll,
    /// Decode multi-byte values as big endian.
    pub big_endian: bool,
    /// Bytes per row; the renderer fits it to the window width, like heh.
    pub columns: usize,
    /// Byte under the mouse pointer, highlighted in both panes.
    pub hovered: Option<usize>,
    /// Other end of the selected range, from visual mode or a mouse drag;
    /// the cursor is the moving end.
    pub anchor: Option<usize>,
    /// A mouse drag is selecting bytes.
    pub dragging: bool,
}

impl HexdumpTab {
    /// The selected byte range, if any.
    pub fn selection(&self) -> Option<std::ops::RangeInclusive<usize>> {
        let (a, b) = (self.anchor?, self.cursor.index);
        Some(a.min(b)..=a.max(b))
    }
}

impl Default for HexdumpTab {
    fn default() -> Self {
        Self {
            cursor: Scroll::default(),
            big_endian: false,
            columns: HEX_COLS,
            hovered: None,
            anchor: None,
            dragging: false,
        }
    }
}

/// Application state.
#[derive(Debug)]
pub struct State {
    /// Is the application running?
    pub running: bool,
    /// Breadcrumb stack of analyzed files; the last one is shown.
    /// Empty only when the initial file failed to load.
    pub files: Vec<Analyzer>,
    /// Why the initial file could not be loaded.
    pub fatal: Option<String>,
    /// Transient message for the status bar, cleared by the next command.
    pub notice: Option<String>,
    /// Selected tab.
    pub tab: Tab,
    /// Row selection of the current tab's main list.
    pub selection: Selection,
    /// Details popup open.
    pub show_details: bool,
    /// Open prompt, if any.
    pub prompt: Option<Prompt>,
    /// Search query; persists after the prompt is confirmed.
    pub search: String,
    /// Jump prompt text.
    pub jump: String,
    /// General tab view state.
    pub general: GeneralTab,
    /// Static tab view state.
    pub static_: StaticTab,
    /// Strings tab view state.
    pub strings: StringsTab,
    /// Dynamic tab view state.
    pub dynamic: DynamicTab,
    /// Hexdump tab view state.
    pub hexdump: HexdumpTab,
    /// Startup options.
    pub options: Options,
    /// The last command and when it ran, for the status bar's key hints.
    pub pressed: Option<(Command, f64)>,
    /// egui context, for repaints from worker threads.
    ctx: egui::Context,
}

impl State {
    /// State with nothing loaded yet.
    fn empty(options: Options, ctx: egui::Context) -> Self {
        Self {
            running: true,
            files: Vec::new(),
            fatal: None,
            notice: None,
            tab: options.tab,
            selection: Selection::default(),
            show_details: false,
            prompt: None,
            search: String::new(),
            jump: String::new(),
            general: GeneralTab::default(),
            static_: StaticTab::default(),
            strings: StringsTab {
                min_length: options.min_strings_len,
                ..StringsTab::default()
            },
            dynamic: DynamicTab::default(),
            hexdump: HexdumpTab::default(),
            options,
            pressed: None,
            ctx,
        }
    }

    /// Analyzes `paths` (the breadcrumb, current file last) and starts
    /// extracting strings of the current one.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(paths: &[PathBuf], options: Options, ctx: egui::Context) -> Self {
        let mut state = Self::empty(options, ctx);
        match paths
            .iter()
            .map(|p| Analyzer::new(p))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(files) => state.files = files,
            Err(e) => state.fatal = Some(e.to_string()),
        }
        state.reset_views();
        state
    }

    /// Analyzes an in-memory file (browser upload or bundled sample).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn from_bytes(name: &str, bytes: Vec<u8>, options: Options, ctx: egui::Context) -> Self {
        let mut state = Self::empty(options, ctx);
        state.open_bytes(name, bytes);
        state
    }

    /// The analyzer of the current file.
    pub fn analyzer(&self) -> Option<&Analyzer> {
        self.files.last()
    }

    /// The dependency under the cursor on the General tab.
    pub fn selected_dependency(&self) -> Option<&Dependency> {
        self.analyzer()?.dependencies.get(self.selection.selected?)
    }

    /// The selected row of the Static table.
    pub fn selected_row(&self) -> Option<&Vec<String>> {
        self.static_.rows.get(self.selection.selected?)
    }

    /// The selected string on the Strings tab.
    pub fn selected_string(&self) -> Option<&Found> {
        let index = *self.strings.rows.get(self.selection.selected?)?;
        self.strings.found.as_ref()?.get(index)
    }

    /// Label/value rows of the details popup.
    pub fn details(&self) -> Vec<(String, String)> {
        match self.tab {
            Tab::StaticAnalysis => self
                .selected_row()
                .map(|row| {
                    let headers = self.static_.info().headers();
                    headers
                        .iter()
                        .zip(row)
                        .map(|(h, v)| (h.to_string(), v.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            Tab::Strings => self
                .selected_string()
                .map(|(offset, s)| {
                    vec![
                        ("Location".into(), format!("{offset} ({offset:#x})")),
                        ("String".into(), s.clone()),
                    ]
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// What `y` copies: the open details popup, or the selection of the current tab.
    pub fn selection_text(&self) -> Option<String> {
        let analyzer = self.analyzer()?;
        let lines = |rows: Vec<(String, String)>| {
            let lines: Vec<String> = rows
                .into_iter()
                .map(|(label, value)| match value.is_empty() {
                    true => label,
                    false => format!("{label}: {value}"),
                })
                .collect();
            (!lines.is_empty()).then(|| lines.join("\n"))
        };
        if self.show_details {
            return match self.tab {
                Tab::DynamicAnalysis => self.dynamic.data().map(|d| d.summary.clone()),
                _ => lines(self.details()),
            };
        }
        match self.tab {
            Tab::General => self.selected_dependency().map(|d| match &d.path {
                Some(path) => path.to_string_lossy().into_owned(),
                None => d.name.clone(),
            }),
            Tab::StaticAnalysis => match self.static_.block {
                Block::Table => self.selected_row().map(|row| row.join("\t")),
                Block::Headers => lines(
                    analyzer
                        .elf
                        .file_headers
                        .items()
                        .into_iter()
                        .map(|row| (row[0].clone(), row[1].clone()))
                        .collect(),
                ),
                Block::Notes => lines(analyzer.elf.notes.rows()),
            },
            Tab::Strings => self.selected_string().map(|(_, s)| s.clone()),
            Tab::DynamicAnalysis => {
                let data = self.dynamic.data()?;
                let text: Vec<&str> = self
                    .dynamic
                    .lines
                    .iter()
                    .map(|&i| data.syscalls[i].text.as_str())
                    .collect();
                (!text.is_empty()).then(|| text.join("\n"))
            }
            // The selected bytes, or the one under the cursor, as hex.
            Tab::Hexdump => {
                let last = analyzer.bytes.len().checked_sub(1)?;
                let cursor = self.hexdump.cursor.index.min(last);
                let range = match self.hexdump.selection() {
                    Some(range) => *range.start().min(&last)..=*range.end().min(&last),
                    None => cursor..=cursor,
                };
                let hex: Vec<String> = analyzer.bytes[range]
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect();
                Some(hex.join(" "))
            }
        }
    }

    /// Key hints for the status bar.
    pub fn key_bindings(&self) -> Vec<(&'static str, &'static str)> {
        let bindings = self.native_key_bindings();
        if cfg!(target_arch = "wasm32") {
            // No quitting a browser tab, and no tracing.
            let dynamic = self.tab == Tab::DynamicAnalysis && self.dynamic.data().is_none();
            return bindings
                .iter()
                .copied()
                .filter(|(keys, _)| *keys != "q" && !(dynamic && *keys == "Enter"))
                .collect();
        }
        bindings.to_vec()
    }

    fn native_key_bindings(&self) -> &'static [(&'static str, &'static str)] {
        if self.files.is_empty() {
            return &[("q", "Quit")];
        }
        match self.tab {
            #[cfg(target_arch = "wasm32")]
            Tab::General => &[
                ("Ctrl+O", "Open file"),
                ("o", "Open docs"),
                ("Enter", "Analyze lib"),
                ("y", "Copy"),
                ("j/k", "Select"),
                ("h/l", "Scroll info"),
                ("Tab", "Next"),
                ("Bksp", "Back"),
            ],
            #[cfg(not(target_arch = "wasm32"))]
            Tab::General => &[
                ("o", "Open docs"),
                ("Enter", "Analyze lib"),
                ("y", "Copy"),
                ("j/k", "Select"),
                ("h/l", "Scroll info"),
                ("Tab", "Next"),
                ("Shift+Tab", "Previous"),
                ("Bksp", "Back"),
                ("q", "Quit"),
            ],
            Tab::StaticAnalysis => &[
                ("Enter", "Details"),
                ("y", "Copy"),
                ("/", "Search"),
                ("h/l", "Table"),
                ("j/k", "Scroll"),
                ("n/p", "Block"),
                ("s", "Readability"),
                ("Tab", "Next"),
                ("q", "Quit"),
            ],
            Tab::DynamicAnalysis => match self.dynamic.trace {
                Trace::Done(_) => &[
                    ("Enter", "Summary"),
                    ("y", "Copy"),
                    ("r", "Re-run"),
                    ("/", "Search"),
                    ("j/k", "Scroll"),
                    ("Tab", "Next"),
                    ("q", "Quit"),
                ],
                _ => &[("Enter", "Run"), ("Tab", "Next"), ("q", "Quit")],
            },
            Tab::Strings => &[
                ("Enter", "Details"),
                ("y", "Copy"),
                ("+", "Longer"),
                ("-", "Shorter"),
                ("/", "Search"),
                ("j/k", "Select"),
                ("Tab", "Next"),
                ("q", "Quit"),
            ],
            Tab::Hexdump => &[
                ("s", "Save"),
                ("g", "Jump"),
                ("/", "Search"),
                ("n", "Endianness"),
                ("h/j/k/l", "Move"),
                ("v", "Visual"),
                ("y", "Copy"),
                ("Tab", "Next"),
                ("q", "Quit"),
            ],
        }
    }

    /// Picks up results from worker threads. Called once per frame.
    pub fn poll_background(&mut self) {
        if let Some(found) = self.strings.pending.as_mut().and_then(Extraction::poll) {
            self.strings.pending = None;
            self.strings.found = Some(found);
            if self.tab == Tab::Strings {
                self.handle_tab();
            }
        }
        if let Some(rx) = &self.dynamic.pending
            && let Ok(result) = rx.try_recv()
        {
            self.dynamic.pending = None;
            self.dynamic.trace = match result {
                Ok(data) => Trace::Done(data),
                Err(e) => {
                    self.notice = Some(format!("Tracing failed: {e}"));
                    Trace::Idle
                }
            };
            if self.tab == Tab::DynamicAnalysis {
                self.handle_tab();
            }
        }
    }

    /// Runs a command and updates the state.
    pub fn run_command(&mut self, command: Command, ctx: &egui::Context) {
        self.notice = None;
        self.pressed = Some((command.clone(), ctx.input(|i| i.time)));
        match command {
            Command::Copy => {
                match self.selection_text() {
                    Some(text) => crate::ui::copy(ctx, text),
                    None => self.notice = Some(String::from("Nothing to copy")),
                }
                // Yanking ends visual mode, as in vim.
                self.hexdump.anchor = None;
            }
            Command::Input(input) => self.input_command(input),
            Command::Exit if self.show_details => self.show_details = false,
            // Like vim, Esc leaves visual mode first.
            Command::Exit if self.tab == Tab::Hexdump && self.hexdump.anchor.is_some() => {
                self.hexdump.anchor = None;
            }
            // A browser tab cannot close itself; quitting would only blank the page.
            Command::Exit => self.running = cfg!(target_arch = "wasm32"),
            Command::Next(ScrollType::Tab, n) => self.select_tab(self.tab.next(n)),
            Command::Previous(ScrollType::Tab, n) => self.select_tab(self.tab.previous(n)),
            command => match self.tab {
                Tab::General => self.general_command(command, ctx),
                Tab::StaticAnalysis => self.static_command(command),
                Tab::DynamicAnalysis => self.dynamic_command(command),
                Tab::Strings => self.strings_command(command),
                Tab::Hexdump => self.hexdump_command(command),
            },
        }
    }

    fn input_command(&mut self, input: InputCommand) {
        match (self.prompt, input) {
            (None, InputCommand::Enter) => {
                let allowed = match self.tab {
                    Tab::General => false,
                    Tab::DynamicAnalysis => self.dynamic.data().is_some(),
                    _ => true,
                };
                if allowed {
                    self.prompt = Some(Prompt::Search);
                }
            }
            // Outside a prompt Backspace goes back (General) or reopens the query.
            (None, InputCommand::Backspace) => {
                if self.tab == Tab::General {
                    self.go_back();
                } else if !self.search.is_empty() {
                    self.prompt = Some(Prompt::Search);
                    self.search.pop();
                    self.handle_tab();
                }
            }
            (None, _) => {}
            (Some(Prompt::Search), InputCommand::Handle(text)) => {
                self.search.push_str(&text);
                self.handle_tab();
            }
            (Some(Prompt::Jump), InputCommand::Handle(text)) => self.jump.push_str(&text),
            (Some(Prompt::Search), InputCommand::Backspace) => {
                if self.search.pop().is_none() {
                    self.prompt = None;
                }
                self.handle_tab();
            }
            (Some(Prompt::Jump), InputCommand::Backspace) => {
                if self.jump.pop().is_none() {
                    self.prompt = None;
                }
            }
            (Some(Prompt::Search), InputCommand::Confirm) => {
                self.prompt = None;
                if self.tab == Tab::Hexdump {
                    self.hex_search();
                }
            }
            (Some(Prompt::Jump), InputCommand::Confirm) => {
                self.prompt = None;
                let text = std::mem::take(&mut self.jump);
                let text = text.trim();
                let offset = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                    Some(hex) => usize::from_str_radix(hex, 16),
                    None => text.parse(),
                };
                match offset {
                    Ok(offset) => self.hexdump.cursor.jump(offset),
                    Err(_) => self.notice = Some(format!("Not an offset: `{text}`")),
                }
            }
            (Some(Prompt::Search), InputCommand::Exit) => {
                self.prompt = None;
                self.search.clear();
                self.handle_tab();
            }
            (Some(Prompt::Jump), InputCommand::Exit) => {
                self.prompt = None;
                self.jump.clear();
            }
            (Some(_), InputCommand::Enter) => {}
        }
    }

    /// Commands specific to the General tab.
    fn general_command(&mut self, command: Command, ctx: &egui::Context) {
        match command {
            Command::Next(ScrollType::Table, n) => self.general.info_scroll.forward(n),
            Command::Previous(ScrollType::Table, n) => self.general.info_scroll.back(n),
            Command::Next(ScrollType::List, n) => self.selection.next(n),
            Command::Previous(ScrollType::List, n) => self.selection.previous(n),
            Command::Top => self.selection.first(),
            Command::Bottom => self.selection.last(),
            Command::ShowDetails => match self.selected_dependency() {
                Some(Dependency {
                    path: Some(path), ..
                }) => self.open_file(path.clone()),
                Some(dependency) if self.analyzer().is_some_and(|a| a.resolved) => {
                    self.notice = Some(format!("Library not found: `{}`", dependency.name));
                }
                Some(dependency) => {
                    self.notice = Some(format!(
                        "`{}` cannot be resolved in the browser; open or drop the library file instead",
                        dependency.name
                    ));
                }
                None => {}
            },
            Command::OpenRepo => ctx.open_url(egui::OpenUrl::new_tab(DOCS_URL)),
            _ => {}
        }
    }

    /// Commands specific to the Static tab.
    fn static_command(&mut self, command: Command) {
        match command {
            Command::Next(ScrollType::Table, n) => self.select_info(self.static_.info_index + n),
            Command::Previous(ScrollType::Table, n) => {
                self.select_info(
                    self.static_
                        .info_index
                        .checked_sub(n)
                        .unwrap_or(INFO_TABS.len() - 1),
                );
            }
            Command::Next(ScrollType::List, n) => match self.static_.block {
                Block::Headers => self.static_.headers_scroll.forward(n),
                Block::Notes => self.static_.notes_scroll.forward(n),
                Block::Table => self.selection.next(n),
            },
            Command::Previous(ScrollType::List, n) => match self.static_.block {
                Block::Headers => self.static_.headers_scroll.back(n),
                Block::Notes => self.static_.notes_scroll.back(n),
                Block::Table => self.selection.previous(n),
            },
            Command::Next(ScrollType::Block, _) => self.static_.block = self.static_.block.next(),
            Command::Previous(ScrollType::Block, _) => {
                self.static_.block = self.static_.block.previous()
            }
            Command::Top => self.selection.first(),
            Command::Bottom => self.selection.last(),
            Command::ShowDetails => {
                self.show_details = !self.show_details && self.selected_row().is_some()
            }
            Command::HumanReadable => {
                if let Some(analyzer) = self.files.last_mut() {
                    analyzer.elf.toggle_readability();
                }
                let selected = self.selection.selected;
                self.handle_tab();
                if let Some(i) = selected {
                    self.selection.select(i);
                }
            }
            _ => {}
        }
    }

    /// Commands specific to the Dynamic tab.
    fn dynamic_command(&mut self, command: Command) {
        let loaded = self.dynamic.data().is_some();
        match command {
            Command::ShowDetails if loaded => self.show_details = !self.show_details,
            Command::ShowDetails | Command::TraceCalls => self.start_trace(),
            Command::Next(ScrollType::List, n) => self.dynamic.scroll.forward(n),
            Command::Previous(ScrollType::List, n) => self.dynamic.scroll.back(n),
            Command::Top => self.dynamic.scroll.back(usize::MAX),
            Command::Bottom => self.dynamic.scroll.forward(usize::MAX),
            _ => {}
        }
    }

    /// Commands specific to the Strings tab.
    fn strings_command(&mut self, command: Command) {
        match command {
            Command::Next(ScrollType::List, n) => self.selection.next(n),
            Command::Previous(ScrollType::List, n) => self.selection.previous(n),
            Command::Top => self.selection.first(),
            Command::Bottom => self.selection.last(),
            Command::ShowDetails => {
                self.show_details = !self.show_details && self.selected_string().is_some()
            }
            Command::Increment => {
                self.strings.min_length += 1;
                self.extract_strings();
            }
            Command::Decrement => {
                self.strings.min_length = (self.strings.min_length - 1).max(1);
                self.extract_strings();
            }
            _ => {}
        }
    }

    /// Commands specific to the Hexdump tab.
    fn hexdump_command(&mut self, command: Command) {
        let columns = self.hexdump.columns;
        let cursor = &mut self.hexdump.cursor;
        match command {
            Command::Next(ScrollType::Table, n) => cursor.forward(n),
            Command::Previous(ScrollType::Table, n) => cursor.back(n),
            Command::Next(ScrollType::List, n) => cursor.forward(n * columns),
            Command::Previous(ScrollType::List, n) => cursor.back(n * columns),
            // binsider saves edits with `s`; this hexdump is a viewer.
            Command::HumanReadable => self.notice = Some(String::from("file is read-only")),
            Command::Top => cursor.back(usize::MAX),
            Command::Bottom => cursor.forward(usize::MAX),
            Command::Next(ScrollType::Block, _) | Command::Previous(ScrollType::Block, _) => {
                self.hexdump.big_endian = !self.hexdump.big_endian;
            }
            Command::Goto => self.prompt = Some(Prompt::Jump),
            Command::Visual => {
                self.hexdump.anchor = match self.hexdump.anchor {
                    Some(_) => None,
                    None => Some(cursor.index),
                };
            }
            _ => {}
        }
    }

    /// Moves the hexdump cursor to the next occurrence of the search query.
    ///
    /// An even-length hex string searches for those bytes, anything else for
    /// the text itself.
    fn hex_search(&mut self) {
        let Some(analyzer) = self.analyzer() else {
            return;
        };
        let query = self.search.trim();
        if query.is_empty() {
            return;
        }
        let needle: Vec<u8> =
            match query.len().is_multiple_of(2) && query.chars().all(|c| c.is_ascii_hexdigit()) {
                true => (0..query.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&query[i..i + 2], 16).ok())
                    .collect(),
                false => query.as_bytes().to_vec(),
            };
        let bytes = &analyzer.bytes;
        let start = self.hexdump.cursor.index + 1;
        let find = |range: std::ops::Range<usize>| {
            range.into_iter().find(|&i| bytes[i..].starts_with(&needle))
        };
        match find(start.min(bytes.len())..bytes.len()).or_else(|| find(0..start.min(bytes.len())))
        {
            Some(offset) => self.hexdump.cursor.jump(offset),
            None => self.notice = Some(format!("Not found: `{query}`")),
        }
    }

    /// Switches the Static tab's ELF table (mouse click on a sub-tab).
    pub fn select_info(&mut self, index: usize) {
        let index = index % INFO_TABS.len();
        if index != self.static_.info_index {
            self.static_.sort = None;
        }
        self.static_.info_index = index;
        self.handle_tab();
    }

    /// Sorts the Static table by a column (header click).
    pub fn sort_static(&mut self, column: usize) {
        self.static_.sort = Sort::cycle(self.static_.sort, column);
        self.handle_tab();
    }

    /// Sorts the strings by location (0) or text (1) (header click).
    pub fn sort_strings(&mut self, column: usize) {
        self.strings.sort = Sort::cycle(self.strings.sort, column);
        self.handle_tab();
    }

    /// Switches tab and rebuilds the per-tab list.
    pub fn select_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.show_details = false;
        self.handle_tab();
    }

    /// Rebuilds the current tab's rows and selection after the tab, the file,
    /// the query or the data changed.
    fn handle_tab(&mut self) {
        let Some(analyzer) = self.files.last() else {
            self.selection = Selection::default();
            return;
        };
        let filter = Filter::new(&self.search);
        let len = match self.tab {
            Tab::General => analyzer.dependencies.len(),
            Tab::StaticAnalysis => {
                let rows = analyzer.elf.rows(self.static_.info());
                self.static_.rows = rows
                    .into_iter()
                    .filter(|row| filter.matches(row.iter().map(String::as_str)))
                    .collect();
                if let Some(sort) = self.static_.sort {
                    let cell =
                        |row: &Vec<String>| row.get(sort.column).cloned().unwrap_or_default();
                    self.static_
                        .rows
                        .sort_by(|a, b| sort.apply(compare_cells(&cell(a), &cell(b))));
                }
                self.static_.rows.len()
            }
            Tab::Strings => {
                let Some(found) = &self.strings.found else {
                    self.strings.rows.clear();
                    self.selection = Selection::new(0);
                    return;
                };
                let mut rows: Vec<usize> = (0..found.len())
                    .filter(|&i| filter.matches([found[i].1.as_str()]))
                    .collect();
                match self.strings.sort {
                    Some(sort) if sort.column == 1 => rows.sort_by(|&a, &b| {
                        sort.apply(found[a].1.to_lowercase().cmp(&found[b].1.to_lowercase()))
                    }),
                    // Extraction yields ascending locations already.
                    Some(Sort {
                        descending: true, ..
                    }) => rows.reverse(),
                    _ => {}
                }
                let widest = rows.iter().map(|&i| found[i].0).max().unwrap_or(0);
                self.strings.offset_digits = widest.to_string().len();
                self.strings.rows = rows;
                self.strings.rows.len()
            }
            Tab::DynamicAnalysis => {
                self.dynamic.lines = match self.dynamic.data() {
                    Some(data) => (0..data.syscalls.len())
                        .filter(|&i| filter.matches([data.syscalls[i].text.as_str()]))
                        .collect(),
                    None => Vec::new(),
                };
                self.dynamic.scroll.back(usize::MAX);
                0
            }
            Tab::Hexdump => 0,
        };
        self.selection = Selection::new(len);
    }

    /// Resets every tab's view for a newly loaded file.
    fn reset_views(&mut self) {
        self.general = GeneralTab::default();
        self.static_ = StaticTab::default();
        self.dynamic = DynamicTab::default();
        self.hexdump = HexdumpTab::default();
        self.show_details = false;
        self.extract_strings();
        self.handle_tab();
    }

    /// Starts extracting strings of the current file in the background.
    fn extract_strings(&mut self) {
        self.strings.found = None;
        self.strings.rows.clear();
        self.strings.pending = self
            .analyzer()
            .map(|a| Extraction::start(a.bytes.clone(), self.strings.min_length, self.ctx.clone()));
        if self.tab == Tab::Strings {
            self.handle_tab();
        }
    }

    /// Runs the current file under ptrace in the background.
    fn start_trace(&mut self) {
        // Browsers cannot ptrace; the tab explains that instead.
        if cfg!(target_arch = "wasm32") || matches!(self.dynamic.trace, Trace::Running) {
            return;
        }
        let Some(path) = self
            .analyzer()
            .map(|a| a.path.to_string_lossy().into_owned())
        else {
            return;
        };
        let mut command = vec![path];
        // The command line arguments belong to the file given on the command line.
        if self.files.len() == 1 {
            command.extend(self.options.arguments.iter().cloned());
        }
        self.dynamic.trace = Trace::Running;
        self.dynamic.lines.clear();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.dynamic.pending = Some(crate::tracer::trace(
                command,
                self.options.trace.clone(),
                self.ctx.clone(),
            ));
        }
    }

    /// Analyzes `path` and pushes it onto the breadcrumb, keeping the current file on error.
    pub fn open_file(&mut self, path: PathBuf) {
        #[cfg(not(target_arch = "wasm32"))]
        let analyzed = Analyzer::new(&path);
        #[cfg(target_arch = "wasm32")]
        let analyzed: crate::error::Result<Analyzer> = Err(std::io::Error::other(format!(
            "cannot read `{}` in the browser",
            path.display()
        ))
        .into());
        self.push(analyzed);
    }

    /// Analyzes an in-memory file and pushes it onto the breadcrumb.
    pub fn open_bytes(&mut self, name: &str, bytes: Vec<u8>) {
        self.push(Analyzer::from_bytes(name, bytes));
    }

    fn push(&mut self, analyzed: crate::error::Result<Analyzer>) {
        match analyzed {
            Ok(analyzer) => {
                self.files.push(analyzer);
                self.fatal = None;
                self.reset_views();
            }
            Err(e) if self.files.is_empty() => self.fatal = Some(e.to_string()),
            Err(e) => self.notice = Some(e.to_string()),
        }
    }

    /// Returns to the previously analyzed file, if any.
    pub fn go_back(&mut self) {
        if self.files.len() > 1 {
            self.files.pop();
            self.reset_views();
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    fn ctx() -> egui::Context {
        egui::Context::default()
    }

    fn ls() -> State {
        State::new(&[PathBuf::from("/bin/ls")], Options::default(), ctx())
    }

    fn wait_for_strings(state: &mut State) {
        for _ in 0..200 {
            state.poll_background();
            if state.strings.found.is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("strings never arrived");
    }

    #[test]
    fn tabs_wrap_both_ways() {
        let mut state = ls();
        assert!(state.analyzer().is_some());
        for _ in 0..Tab::ALL.len() {
            state.run_command(Command::Next(ScrollType::Tab, 1), &ctx());
        }
        assert_eq!(state.tab, Tab::General);
        state.run_command(Command::Previous(ScrollType::Tab, 1), &ctx());
        assert_eq!(state.tab, Tab::Hexdump);
        assert_eq!(Tab::Hexdump.next(1), Tab::General);
    }

    #[test]
    fn breadcrumb_push_and_pop() {
        let mut state = ls();
        let libc = state
            .analyzer()
            .unwrap()
            .dependencies
            .iter()
            .position(|d| d.name.starts_with("libc.so"))
            .expect("libc dependency");
        state.selection.select(libc);
        let libc_path = state.selected_dependency().unwrap().path.clone().unwrap();
        state.run_command(Command::ShowDetails, &ctx());
        assert_eq!(state.files.len(), 2);
        assert_eq!(state.analyzer().unwrap().path, libc_path);
        assert!(state.notice.is_none());
        state.run_command(Command::Input(InputCommand::Backspace), &ctx());
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.analyzer().unwrap().path, Path::new("/bin/ls"));
        state.run_command(Command::Input(InputCommand::Backspace), &ctx());
        assert_eq!(state.files.len(), 1);
    }

    #[test]
    fn bad_file_keeps_current_analyzer() {
        let mut state = ls();
        state.open_file(PathBuf::from("/etc/hostname"));
        assert_eq!(state.files.len(), 1);
        assert!(state.notice.is_some());
    }

    #[test]
    fn initial_failure_is_reported_and_kept() {
        let mut state = State::new(&[PathBuf::from("/nonexistent")], Options::default(), ctx());
        assert!(state.files.is_empty());
        assert!(state.fatal.is_some());
        assert_eq!(state.key_bindings(), vec![("q", "Quit")]);
        state.run_command(Command::Next(ScrollType::Tab, 1), &ctx());
        assert!(state.fatal.is_some());
    }

    #[test]
    fn static_tables_search_and_details() {
        let mut state = ls();
        state.select_tab(Tab::StaticAnalysis);
        assert_eq!(state.static_.info(), Info::ProgramHeaders);
        assert!(!state.static_.rows.is_empty());
        state.run_command(Command::Next(ScrollType::Table, 2), &ctx());
        assert_eq!(state.static_.info(), Info::Symbols);
        state.run_command(Command::Previous(ScrollType::Table, 3), &ctx());
        assert_eq!(state.static_.info(), Info::Relocations);
        state.run_command(Command::Next(ScrollType::Table, 1), &ctx());
        assert_eq!(state.static_.info(), Info::ProgramHeaders);
        let all = state.static_.rows.len();
        state.run_command(Command::Input(InputCommand::Enter), &ctx());
        assert_eq!(state.prompt, Some(Prompt::Search));
        state.run_command(Command::Input(InputCommand::Handle("LOAD".into())), &ctx());
        assert!(!state.static_.rows.is_empty() && state.static_.rows.len() < all);
        assert!(state.static_.rows.iter().all(|r| r[0].contains("LOAD")));
        state.run_command(Command::Input(InputCommand::Confirm), &ctx());
        assert_eq!(state.prompt, None);
        assert_eq!(state.search, "LOAD");
        state.run_command(Command::ShowDetails, &ctx());
        assert!(state.show_details);
        state.run_command(Command::Exit, &ctx());
        assert!(!state.show_details && state.running);
        // Backspace outside the prompt reopens it and deletes one character.
        state.run_command(Command::Input(InputCommand::Backspace), &ctx());
        assert_eq!(
            (state.prompt, state.search.as_str()),
            (Some(Prompt::Search), "LOA")
        );
        state.run_command(Command::Input(InputCommand::Exit), &ctx());
        assert_eq!((state.prompt, state.search.as_str()), (None, ""));
        assert_eq!(state.static_.rows.len(), all);
        // Readability toggle keeps the selection.
        state.run_command(Command::Bottom, &ctx());
        let last = state.selection.selected;
        state.run_command(Command::HumanReadable, &ctx());
        assert_eq!(state.selection.selected, last);
        // Block cycling.
        state.run_command(Command::Next(ScrollType::Block, 1), &ctx());
        assert_eq!(state.static_.block, Block::Headers);
        state.run_command(Command::Next(ScrollType::List, 2), &ctx());
        assert_eq!(state.static_.headers_scroll.index, 2);
        state.run_command(Command::Previous(ScrollType::Block, 1), &ctx());
        assert_eq!(state.static_.block, Block::Table);
    }

    #[test]
    fn copy_takes_the_selection() {
        let ctx = ctx();
        let mut state = ls();
        let path = state.selection_text().expect("a dependency is selected");
        assert!(path.starts_with('/'), "{path}");

        state.select_tab(Tab::StaticAnalysis);
        let row = state.selected_row().expect("a row is selected").join("\t");
        assert_eq!(state.selection_text(), Some(row));
        state.run_command(Command::ShowDetails, &ctx);
        let details = state.selection_text().expect("details");
        assert!(
            details.lines().count() > 1 && details.contains(": "),
            "{details}"
        );
        state.run_command(Command::Exit, &ctx);
        state.run_command(Command::Previous(ScrollType::Block, 1), &ctx);
        let notes = state.selection_text().expect("notes");
        assert!(notes.starts_with("Notes in "), "{notes}");

        state.select_tab(Tab::Strings);
        wait_for_strings(&mut state);
        let string = state.selected_string().unwrap().1.clone();
        assert_eq!(state.selection_text(), Some(string));

        // The hexdump copies the byte under the cursor, or a visual selection.
        state.select_tab(Tab::Hexdump);
        assert_eq!(state.selection_text().as_deref(), Some("7F"));
        state.run_command(Command::Visual, &ctx);
        state.run_command(Command::Next(ScrollType::Table, 3), &ctx);
        assert_eq!(state.hexdump.selection(), Some(0..=3));
        assert_eq!(state.selection_text().as_deref(), Some("7F 45 4C 46"));
        state.run_command(Command::Copy, &ctx);
        assert_eq!(state.hexdump.selection(), None, "yank leaves visual mode");
        state.run_command(Command::Visual, &ctx);
        state.run_command(Command::Exit, &ctx);
        assert!(state.running, "Esc leaves visual mode before quitting");
        assert_eq!(state.hexdump.selection(), None);
    }

    #[test]
    fn sorting_cycles_and_compares_values() {
        let first = Sort::cycle(None, 2);
        assert_eq!(
            first,
            Some(Sort {
                column: 2,
                descending: false
            })
        );
        let second = Sort::cycle(first, 2);
        assert_eq!(
            second,
            Some(Sort {
                column: 2,
                descending: true
            })
        );
        assert_eq!(Sort::cycle(second, 2), None);
        assert_eq!(
            Sort::cycle(second, 0),
            Some(Sort {
                column: 0,
                descending: false
            })
        );

        use std::cmp::Ordering::{Greater, Less};
        assert_eq!(compare_cells("0x9", "0x10"), Less);
        assert_eq!(compare_cells("100", "20"), Greater);
        assert_eq!(compare_cells("2.0 KiB", "900 B"), Greater);
        assert_eq!(compare_cells("12", "abc"), Less);
        assert_eq!(compare_cells("Beta", "alpha"), Greater);
    }

    #[test]
    fn header_clicks_sort_tables() {
        let mut state = ls();
        state.select_tab(Tab::StaticAnalysis);
        state.select_info(1); // Section headers.
        let original = state.static_.rows.clone();
        // Name column, descending after two clicks.
        state.sort_static(0);
        state.sort_static(0);
        let names: Vec<&str> = state.static_.rows.iter().map(|r| r[0].as_str()).collect();
        let mut expected = names.clone();
        expected.sort_by(|a, b| compare_cells(b, a));
        assert_eq!(names, expected);
        state.sort_static(0);
        assert_eq!(
            state.static_.rows, original,
            "third click restores the file order"
        );
        state.select_info(2);
        assert_eq!(state.static_.sort, None, "another table starts unsorted");

        state.select_tab(Tab::Strings);
        wait_for_strings(&mut state);
        state.sort_strings(1);
        let found = state.strings.found.as_ref().unwrap();
        let texts: Vec<String> = state
            .strings
            .rows
            .iter()
            .map(|&i| found[i].1.to_lowercase())
            .collect();
        assert!(texts.is_sorted());
        state.sort_strings(0);
        let found = state.strings.found.as_ref().unwrap();
        let offsets: Vec<u64> = state.strings.rows.iter().map(|&i| found[i].0).collect();
        assert!(offsets.is_sorted());
    }

    #[test]
    fn filter_ignores_case() {
        let filter = Filter::new("MalLoc");
        assert!(filter.matches(["x", "__libc_malloc"]));
        assert!(filter.matches(["ÜBER_MALLOC"]));
        assert!(!filter.matches(["free", "ÜBER"]));
        assert!(Filter::new("").matches([]));
        assert!(Filter::new("ü").matches(["ÜBER"]));
    }

    #[test]
    fn strings_load_filter_and_length() {
        let mut state = ls();
        state.select_tab(Tab::Strings);
        wait_for_strings(&mut state);
        let found = state.strings.found.as_ref().unwrap().len();
        assert!(found > 0);
        assert_eq!(state.selection.len, found);
        state.run_command(Command::Input(InputCommand::Enter), &ctx());
        state.run_command(Command::Input(InputCommand::Handle("GLIBC".into())), &ctx());
        assert!(state.selection.len < found);
        state.run_command(Command::Input(InputCommand::Exit), &ctx());
        state.run_command(Command::Decrement, &ctx());
        assert_eq!(state.strings.min_length, DEFAULT_MIN_STRING_LEN - 1);
        assert!(state.strings.found.is_none());
        wait_for_strings(&mut state);
        assert!(state.strings.found.as_ref().unwrap().len() >= found);
    }

    #[test]
    fn hexdump_cursor_jump_and_search() {
        let mut state = ls();
        state.select_tab(Tab::Hexdump);
        state.run_command(Command::Next(ScrollType::List, 2), &ctx());
        state.run_command(Command::Next(ScrollType::Table, 3), &ctx());
        assert_eq!(state.hexdump.cursor.index, 35);
        state.run_command(Command::Goto, &ctx());
        assert_eq!(state.prompt, Some(Prompt::Jump));
        state.run_command(Command::Input(InputCommand::Handle("0x10".into())), &ctx());
        state.run_command(Command::Input(InputCommand::Confirm), &ctx());
        assert_eq!(state.hexdump.cursor.index, 16);
        // "ELF" follows the 0x7f magic byte at offset 1.
        state.run_command(Command::Top, &ctx());
        state.run_command(Command::Input(InputCommand::Enter), &ctx());
        state.run_command(Command::Input(InputCommand::Handle("ELF".into())), &ctx());
        state.run_command(Command::Input(InputCommand::Confirm), &ctx());
        assert_eq!(state.hexdump.cursor.index, 1);
        state.search = "7f454c46".into();
        state.hexdump.cursor.index = 5;
        state.hex_search();
        assert_eq!(state.hexdump.cursor.index, 0);
        assert!(!state.hexdump.big_endian);
        state.run_command(Command::Next(ScrollType::Block, 1), &ctx());
        assert!(state.hexdump.big_endian);
    }

    #[test]
    fn options_and_breadcrumb_from_command_line() {
        let options = Options {
            tab: Tab::Strings,
            min_strings_len: 4,
            ..Options::default()
        };
        let state = State::new(
            &[PathBuf::from("/bin/ls"), PathBuf::from("/bin/cat")],
            options,
            ctx(),
        );
        assert_eq!(state.tab, Tab::Strings);
        assert_eq!(state.strings.min_length, 4);
        assert_eq!(state.files.len(), 2);
        assert_eq!(state.analyzer().unwrap().file.name, "cat");
    }

    #[test]
    fn uploaded_sample_cannot_open_libraries() {
        let sample = include_bytes!("../assets/sample").to_vec();
        let mut state = State::from_bytes("sample", sample, Options::default(), ctx());
        assert!(state.fatal.is_none());
        assert!(state.selection.len >= 3);
        state.run_command(Command::ShowDetails, &ctx());
        assert_eq!(state.files.len(), 1);
        assert!(
            state
                .notice
                .as_deref()
                .is_some_and(|n| n.contains("browser"))
        );
        // A bad upload replaces nothing and reports.
        state.open_bytes("junk", b"junk".to_vec());
        assert_eq!(state.files.len(), 1);
        assert!(state.notice.is_some());
        let empty = State::from_bytes("junk", b"junk".to_vec(), Options::default(), ctx());
        assert!(empty.fatal.is_some());
    }

    #[test]
    fn hexdump_rows_follow_columns_and_save_is_read_only() {
        let mut state = ls();
        state.select_tab(Tab::Hexdump);
        state.hexdump.columns = 32;
        state.run_command(Command::Next(ScrollType::List, 1), &ctx());
        assert_eq!(state.hexdump.cursor.index, 32);
        state.run_command(Command::HumanReadable, &ctx());
        assert_eq!(state.notice.as_deref(), Some("file is read-only"));
    }

    #[test]
    fn exit_and_scroll() {
        let mut state = ls();
        state.run_command(Command::Next(ScrollType::Table, 3), &ctx());
        assert_eq!(state.general.info_scroll.index, 3);
        state.run_command(Command::Previous(ScrollType::Table, 5), &ctx());
        assert_eq!(state.general.info_scroll.index, 0);
        state.run_command(Command::Bottom, &ctx());
        assert_eq!(state.selection.selected, state.selection.len.checked_sub(1));
        assert!(state.selection.take_moved());
        state.run_command(Command::Exit, &ctx());
        assert!(!state.running);
    }
}
