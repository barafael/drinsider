//! eframe application wrapper.

use crate::{command, command::Command, state::State, ui};
use std::sync::mpsc;

/// A file picked or dropped in the browser: name and contents.
type Loaded = (String, Vec<u8>);

/// The drinsider window.
pub struct DrinsiderApp {
    state: State,
    /// Files whose contents are still being read (browser drops and picks).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    loaded_tx: mpsc::Sender<Loaded>,
    loaded_rx: mpsc::Receiver<Loaded>,
}

impl DrinsiderApp {
    /// Wraps an initial state.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        make_state: impl FnOnce(egui::Context) -> State,
    ) -> Self {
        ui::install(&cc.egui_ctx);
        let (loaded_tx, loaded_rx) = mpsc::channel();
        Self {
            state: make_state(cc.egui_ctx.clone()),
            loaded_tx,
            loaded_rx,
        }
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
}

impl eframe::App for DrinsiderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.state.poll_background();
        self.handle_drops(&ctx);
        let mut keys = command::Keys::take(&ctx);
        while let Some(cmd) = keys.next(self.state.prompt.is_some()) {
            match cmd {
                Command::OpenFile => {
                    #[cfg(target_arch = "wasm32")]
                    crate::web::pick_file(self.loaded_tx.clone(), ctx.clone());
                }
                cmd => self.state.run_command(cmd, &ctx),
            }
        }
        if !self.state.running {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        ui::render(&mut self.state, ui);
    }
}
