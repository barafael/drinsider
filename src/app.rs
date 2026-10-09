//! eframe application wrapper.

use crate::{command, command::Command, state::State, ui};
use std::sync::mpsc;

/// Storage key of the theme setting.
const THEME_KEY: &str = "theme";

/// A file picked or dropped in the browser: name and contents.
type Loaded = (String, Vec<u8>);

/// The drinsider window.
pub struct DrinsiderApp {
    state: State,
    /// For saving settings, which happens outside a frame.
    ctx: egui::Context,
    /// Files whose contents are still being read (browser drops and picks).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    loaded_tx: mpsc::Sender<Loaded>,
    loaded_rx: mpsc::Receiver<Loaded>,
    /// The desktop's light/dark preference.
    #[cfg(target_os = "linux")]
    system_theme: crate::portal::SystemTheme,
    /// Saves the window's size and position.
    #[cfg(not(target_arch = "wasm32"))]
    window: Option<crate::geometry::Tracker>,
}

impl DrinsiderApp {
    /// Wraps an initial state.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        make_state: impl FnOnce(egui::Context) -> State,
    ) -> Self {
        let theme = cc
            .storage
            .and_then(|storage| storage.get_string(THEME_KEY))
            .map(|stored| ui::theme::Settings::from_stored(&stored))
            .unwrap_or_default();
        ui::theme::install(&cc.egui_ctx, theme);
        let (loaded_tx, loaded_rx) = mpsc::channel();
        Self {
            state: make_state(cc.egui_ctx.clone()),
            ctx: cc.egui_ctx.clone(),
            loaded_tx,
            loaded_rx,
            #[cfg(target_os = "linux")]
            system_theme: crate::portal::SystemTheme::watch(cc.egui_ctx.clone()),
            #[cfg(not(target_arch = "wasm32"))]
            window: None,
        }
    }

    /// Remembers the window's geometry between runs.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_window(mut self, tracker: crate::geometry::Tracker) -> Self {
        self.window = Some(tracker);
        self
    }

    /// Opens dropped files: by path on the desktop, by contents in the browser.
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            #[cfg(not(target_arch = "wasm32"))]
            self.state.open_file(file.path().to_path_buf());
            #[cfg(target_arch = "wasm32")]
            {
                let tx = self.loaded_tx.clone();
                let ctx = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let name = file
                        .path()
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| String::from("dropped file"));
                    if let Ok(bytes) = file.bytes_async().await {
                        let _ = tx.send((name, bytes));
                        ctx.request_repaint();
                    }
                });
            }
        }
        while let Ok((name, bytes)) = self.loaded_rx.try_recv() {
            self.state.open_bytes(&name, bytes);
        }
    }

    /// Runs a command from a key press or a clicked key hint.
    fn run(&mut self, command: Command, ctx: &egui::Context) {
        match command {
            Command::OpenFile => {
                #[cfg(target_arch = "wasm32")]
                crate::web::pick_file(self.loaded_tx.clone(), ctx.clone());
            }
            command => self.state.run_command(command, ctx),
        }
    }
}

impl eframe::App for DrinsiderApp {
    /// Remembers the theme and the window's geometry.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(THEME_KEY, ui::theme::current(&self.ctx).to_stored());
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(window) = &self.window {
            window.save(&self.ctx);
        }
    }

    /// Supplies the system theme where winit does not know it.
    #[cfg(target_os = "linux")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if raw_input.system_theme.is_none() {
            raw_input.system_theme = self.system_theme.get();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(window) = &mut self.window {
            window.observe(&ctx);
        }
        self.state.poll_background();
        self.handle_drops(&ctx);
        // An open menu takes Escape to close itself; it must not also quit.
        if egui::Popup::is_any_open(&ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            egui::Popup::close_all(&ctx);
        }
        let mut keys = command::Keys::take(&ctx);
        while let Some(command) = keys.next(self.state.prompt.is_some()) {
            self.run(command, &ctx);
        }
        if !self.state.running {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if let Some(command) = ui::render(&mut self.state, ui) {
            self.run(command, &ctx);
            ctx.request_repaint();
        }
    }
}
