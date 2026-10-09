//! `strings(1)`-style extraction: runs of printable ASCII, like the
//! rust-strings crate binsider uses. A worker thread does it on the desktop;
//! the browser has no threads here, so it scans a slice per frame instead.

use std::sync::Arc;

/// A printable string and its byte offset.
pub type Found = (u64, String);

/// Milliseconds the browser spends scanning per frame, to stay responsive.
#[cfg(target_arch = "wasm32")]
const FRAME_BUDGET_MS: f64 = 12.0;

/// rust-strings' printable set: visible ASCII, space, tab, LF and CR.
fn printable(byte: u8) -> bool {
    matches!(byte, b' '..=b'~' | b'\t' | b'\n' | b'\r')
}

/// A resumable scan over a file.
#[derive(Debug)]
struct Scan {
    bytes: Arc<[u8]>,
    min_length: usize,
    position: usize,
    found: Vec<Found>,
}

impl Scan {
    fn new(bytes: Arc<[u8]>, min_length: usize) -> Self {
        Self {
            bytes,
            min_length: min_length.max(1),
            position: 0,
            found: Vec::new(),
        }
    }

    /// Scans at least `budget` bytes (finishing the run it ends in) and
    /// returns whether the whole file is done.
    fn step(&mut self, budget: usize) -> bool {
        let bytes = &self.bytes[..];
        let stop = self.position.saturating_add(budget);
        while self.position < bytes.len() && self.position < stop {
            let rest = &bytes[self.position..];
            let Some(skip) = rest.iter().position(|&b| printable(b)) else {
                self.position = bytes.len();
                break;
            };
            let start = self.position + skip;
            let len = bytes[start..]
                .iter()
                .position(|&b| !printable(b))
                .unwrap_or(bytes.len() - start);
            if len >= self.min_length {
                let text = bytes[start..start + len]
                    .iter()
                    .map(|&b| b as char)
                    .collect();
                self.found.push((start as u64, text));
            }
            self.position = start + len;
        }
        self.position >= bytes.len()
    }

    /// Fraction of the file scanned.
    #[cfg(target_arch = "wasm32")]
    fn progress(&self) -> f32 {
        if self.bytes.is_empty() {
            1.0
        } else {
            self.position as f32 / self.bytes.len() as f32
        }
    }
}

/// Strings of at least `min_length` bytes, being extracted.
#[derive(Debug)]
pub struct Extraction {
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<Vec<Found>>,
    #[cfg(target_arch = "wasm32")]
    scan: Scan,
    #[cfg(target_arch = "wasm32")]
    ctx: egui::Context,
}

impl Extraction {
    /// Starts extracting; `ctx` is repainted when there is progress.
    pub fn start(bytes: Arc<[u8]>, min_length: usize, ctx: egui::Context) -> Self {
        let scan = Scan::new(bytes, min_length);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut scan = scan;
                scan.step(usize::MAX);
                // The receiver may be gone if the user moved on; nothing to do then.
                let _ = tx.send(scan.found);
                ctx.request_repaint();
            });
            Self { rx }
        }
        #[cfg(target_arch = "wasm32")]
        {
            ctx.request_repaint();
            Self { scan, ctx }
        }
    }

    /// Makes progress and returns the strings once all are found. Called once per frame.
    pub fn poll(&mut self) -> Option<Vec<Found>> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.rx.try_recv().ok();
        #[cfg(target_arch = "wasm32")]
        {
            let deadline = js_sys::Date::now() + FRAME_BUDGET_MS;
            while js_sys::Date::now() < deadline {
                if self.scan.step(256 << 10) {
                    return Some(std::mem::take(&mut self.scan.found));
                }
            }
            self.ctx.request_repaint();
            None
        }
    }

    /// Fraction done, where it is known.
    pub fn progress(&self) -> Option<f32> {
        #[cfg(not(target_arch = "wasm32"))]
        return None;
        #[cfg(target_arch = "wasm32")]
        Some(self.scan.progress())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(bytes: &[u8], min_length: usize, budget: usize) -> Vec<Found> {
        let mut scan = Scan::new(bytes.into(), min_length);
        while !scan.step(budget) {}
        scan.found
    }

    fn rust_strings(bytes: &[u8], min_length: usize) -> Vec<Found> {
        let config = rust_strings::BytesConfig::new(bytes.to_vec()).with_min_length(min_length);
        let found = rust_strings::strings(&config).expect("rust-strings");
        found.into_iter().map(|(s, offset)| (offset, s)).collect()
    }

    #[test]
    fn same_as_rust_strings_in_any_slicing() {
        let sample = include_bytes!("../assets/sample");
        for min_length in [1, 4, 15] {
            let expected = rust_strings(sample, min_length);
            assert!(!expected.is_empty());
            for budget in [1, 7, 4096, usize::MAX] {
                assert_eq!(
                    extract(sample, min_length, budget),
                    expected,
                    "min {min_length} budget {budget}"
                );
            }
        }
    }

    #[test]
    fn edges() {
        assert!(extract(b"", 1, 1).is_empty());
        assert_eq!(extract(b"abc", 3, 1), [(0, "abc".to_string())]);
        assert_eq!(
            extract(b"\0ab\tc\0de\0", 2, 2),
            [(1, "ab\tc".into()), (6, "de".into())]
        );
    }
}
