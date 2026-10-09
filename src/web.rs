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
        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("no document");
        let canvas = document
            .get_element_by_id("drinsider_canvas")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("no canvas");
        claim_shortcuts();
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(Box::new(crate::app::DrinsiderApp::new(cc, |ctx| {
                        crate::state::State::from_bytes(
                            "sample",
                            SAMPLE.to_vec(),
                            Default::default(),
                            ctx,
                        )
                    })))
                }),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.remove(),
                Err(e) => {
                    loading.set_text_content(Some(&format!("drinsider failed to start: {e:?}")))
                }
            }
        }
    });
}

/// Keeps binsider's Ctrl shortcuts (search, page down, page up) from also
/// opening the browser's find bar, bookmark dialog or page source.
/// eframe only claims a fixed few Ctrl shortcuts itself; the app is the
/// whole page, so these are claimed page-wide.
fn claim_shortcuts() {
    let on_key = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(|event: web_sys::KeyboardEvent| {
        if event.ctrl_key() && matches!(event.key().as_str(), "f" | "d" | "u") {
            event.prevent_default();
        }
    });
    if let Some(window) = web_sys::window() {
        // Capture phase: eframe stops key events from bubbling up to the window.
        let _ = window.add_event_listener_with_callback_and_bool(
            "keydown",
            on_key.as_ref().unchecked_ref(),
            true,
        );
    }
    on_key.forget();
}

/// Opens the browser's file dialog; the chosen file arrives on `tx`.
pub fn pick_file(tx: mpsc::Sender<(String, Vec<u8>)>, ctx: egui::Context) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
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
        let Some(file) = picked.files().and_then(|files| files.get(0)) else {
            return;
        };
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
