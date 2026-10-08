//! `strings(1)`-style extraction, on a worker thread on the desktop.

use std::sync::mpsc;

/// A printable string and its byte offset.
pub type Found = (u64, String);

/// Result of one extraction.
pub type Extracted = Result<Vec<Found>, String>;

fn run(bytes: Vec<u8>, min_length: usize) -> Extracted {
    let config = rust_strings::BytesConfig::new(bytes).with_min_length(min_length);
    rust_strings::strings(&config)
        .map(|found| found.into_iter().map(|(s, offset)| (offset, s)).collect())
        .map_err(|e| e.to_string())
}

/// Extracts strings of at least `min_length` bytes.
///
/// The result arrives on the returned channel; `ctx` is repainted when it
/// does. Browsers have no threads here, so the web build extracts inline.
pub fn extract(bytes: Vec<u8>, min_length: usize, ctx: egui::Context) -> mpsc::Receiver<Extracted> {
    let (tx, rx) = mpsc::channel();
    let job = move || {
        // The receiver may be gone if the user moved on; nothing to do then.
        let _ = tx.send(run(bytes, min_length));
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(job);
    #[cfg(target_arch = "wasm32")]
    job();
    rx
}
