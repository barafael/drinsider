//! Keyboard-driven cursors.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

/// Selection over `len` rows that wraps at both ends, as in binsider.
///
/// The rows themselves live with the data that owns them; this only tracks
/// which one is selected and whether the keyboard moved it since the renderer
/// last looked (so the view can scroll to it once).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// Number of rows.
    pub len: usize,
    /// Index of the selected row, if any.
    pub selected: Option<usize>,
    moved: bool,
}

impl Selection {
    /// Selection over `len` rows with the first one selected.
    pub fn new(len: usize) -> Self {
        Self {
            len,
            selected: (len > 0).then_some(0),
            moved: true,
        }
    }

    fn set(&mut self, index: Option<usize>) {
        self.selected = index.filter(|i| *i < self.len);
        self.moved = true;
    }

    /// Selects a row directly (mouse click).
    pub fn select(&mut self, index: usize) {
        self.set(Some(index));
    }

    /// Selects the first row.
    pub fn first(&mut self) {
        self.set((self.len > 0).then_some(0));
    }

    /// Selects the last row.
    pub fn last(&mut self) {
        self.set(self.len.checked_sub(1));
    }

    /// Moves `amount` rows down, wrapping to the top.
    pub fn next(&mut self, amount: usize) {
        self.set(match self.selected {
            Some(i) if i.saturating_add(amount) < self.len => Some(i + amount),
            _ => (self.len > 0).then_some(0),
        });
    }

    /// Moves `amount` rows up, wrapping to the bottom.
    pub fn previous(&mut self, amount: usize) {
        self.set(match self.selected {
            Some(0) | None => self.len.checked_sub(1),
            Some(i) => Some(i.saturating_sub(amount)),
        });
    }

    /// Whether the keyboard moved the selection since the last call.
    pub fn take_moved(&mut self) -> bool {
        std::mem::take(&mut self.moved)
    }
}

/// Scroll offset in rows for a text block, moved with h/l.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scroll {
    /// First visible row.
    pub index: usize,
    moved: bool,
}

impl Scroll {
    /// Scrolls down by `amount` rows (clamped by the renderer).
    pub fn forward(&mut self, amount: usize) {
        self.index = self.index.saturating_add(amount);
        self.moved = true;
    }

    /// Jumps to `index` (clamped by the renderer).
    pub fn jump(&mut self, index: usize) {
        self.index = index;
        self.moved = true;
    }

    /// Scrolls up by `amount` rows.
    pub fn back(&mut self, amount: usize) {
        self.index = self.index.saturating_sub(amount);
        self.moved = true;
    }

    /// Clamps to the last row and reports whether the keyboard moved it.
    pub fn take_moved(&mut self, len: usize) -> bool {
        self.index = self.index.min(len.saturating_sub(1));
        std::mem::take(&mut self.moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps() {
        let mut sel = Selection::new(3);
        assert_eq!(sel.selected, Some(0));
        assert!(sel.take_moved());
        assert!(!sel.take_moved());
        sel.next(1);
        sel.next(1);
        assert_eq!(sel.selected, Some(2));
        sel.next(1);
        assert_eq!(sel.selected, Some(0));
        sel.previous(1);
        assert_eq!(sel.selected, Some(2));
        sel.previous(5);
        assert_eq!(sel.selected, Some(0));
        sel.next(5);
        assert_eq!(sel.selected, Some(0));
        sel.last();
        assert_eq!(sel.selected, Some(2));
        sel.first();
        assert_eq!(sel.selected, Some(0));
        sel.select(7);
        assert_eq!(sel.selected, None);
        assert!(sel.take_moved());
    }

    #[test]
    fn empty_selection() {
        let mut sel = Selection::default();
        sel.next(1);
        sel.previous(1);
        sel.first();
        sel.last();
        assert_eq!(sel.selected, None);
    }

    #[test]
    fn scroll_clamps() {
        let mut scroll = Scroll::default();
        scroll.forward(20);
        assert!(scroll.take_moved(13));
        assert_eq!(scroll.index, 12);
        assert!(!scroll.take_moved(13));
        scroll.back(30);
        assert_eq!(scroll.index, 0);
    }
}
