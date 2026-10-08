//! Browser glue: entry point and file picker.

use std::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Sample binary shown until the user opens their own.
const SAMPLE: &[u8] = include_bytes!("../assets/sample");

/// Starts the app on the page's canvas.
pub fn start() {
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().and_then(|w| w.document()).expect("no document");
        let canvas = document
            .get_element_by_id("drinsider_canvas")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("no canvas");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(Box::new(crate::app::DrinsiderApp::new(cc, |ctx| {
                        crate::state::State::from_bytes("sample", SAMPLE.to_vec(), Default::default(), ctx)
                    })))
                }),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.remove(),
                Err(e) => loading.set_text_content(Some(&format!("drinsider failed to start: {e:?}"))),
            }
        }
    });
}

/// Opens the browser's file dialog; the chosen file arrives on `tx`.
pub fn pick_file(tx: mpsc::Sender<(String, Vec<u8>)>, ctx: egui::Context) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
    let Some(input) = document
        .create_element("input")
        .ok()
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return;
    };
    input.set_type("file");
    let picked = input.clone();
    let on_change = Closure::once(move || {
        let Some(file) = picked.files().and_then(|files| files.get(0)) else { return };
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(buffer) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                let _ = tx.send((file.name(), bytes));
                ctx.request_repaint();
            }
        });
    });
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input.click();
}
