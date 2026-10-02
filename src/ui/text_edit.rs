//! Client-only UTF-8 caret and selection. No authority input is published here.

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextEdit {
    value: String,
    anchor: usize,
    caret: usize,
}

impl TextEdit {
    pub fn new(value: String) -> Self {
        let caret = value.len();
        Self {
            value,
            anchor: caret,
            caret,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// UTF-8 byte boundary used by the measured native caret projection.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Move to a validated UTF-8 byte boundary. Invalid pointer-derived input
    /// preserves both the caret and selection anchor.
    pub fn place_caret(&mut self, offset: usize, extend: bool) -> bool {
        if offset > self.value.len() || !self.value.is_char_boundary(offset) {
            return false;
        }
        self.caret = offset;
        if !extend {
            self.anchor = offset;
        }
        true
    }

    pub fn selection(&self) -> std::ops::Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    /// A clipped presentation snapshot retains the visible part of selection.
    pub(crate) fn viewport(&self, range: std::ops::Range<usize>) -> Self {
        assert!(self.value.is_char_boundary(range.start));
        assert!(self.value.is_char_boundary(range.end));
        Self {
            value: self.value[range.clone()].to_owned(),
            anchor: self.anchor.clamp(range.start, range.end) - range.start,
            caret: self.caret.clamp(range.start, range.end) - range.start,
        }
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.value.len();
    }

    pub fn move_caret(&mut self, direction: i32, extend: bool) {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            if direction < 0 {
                selection.start
            } else {
                selection.end
            }
        } else if direction < 0 {
            self.value[..self.caret]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i)
        } else {
            self.value[self.caret..]
                .chars()
                .next()
                .map_or(self.caret, |c| self.caret + c.len_utf8())
        };
        self.caret = next;
        if !extend {
            self.anchor = next;
        }
    }

    pub fn move_edge(&mut self, end: bool, extend: bool) {
        self.caret = if end { self.value.len() } else { 0 };
        if !extend {
            self.anchor = self.caret;
        }
    }

    pub fn move_word(&mut self, direction: i32, extend: bool) {
        let class = |character: char| {
            if character.is_whitespace() {
                0
            } else if character.is_alphanumeric() || character == '_' {
                1
            } else {
                2
            }
        };
        let mut boundary = self.caret;
        if direction < 0 {
            let mut characters = self.value[..boundary].char_indices().rev().peekable();
            while characters
                .peek()
                .is_some_and(|(_, character)| class(*character) == 0)
            {
                boundary = characters.next().unwrap().0;
            }
            if let Some((_, character)) = characters.peek() {
                let group = class(*character);
                while characters
                    .peek()
                    .is_some_and(|(_, character)| class(*character) == group)
                {
                    boundary = characters.next().unwrap().0;
                }
            }
        } else {
            let mut characters = self.value[boundary..].char_indices().peekable();
            if let Some((_, character)) = characters.peek() {
                let group = class(*character);
                while characters
                    .peek()
                    .is_some_and(|(_, character)| class(*character) == group)
                {
                    let (offset, character) = characters.next().unwrap();
                    boundary = self.caret + offset + character.len_utf8();
                }
            }
            while characters
                .peek()
                .is_some_and(|(_, character)| class(*character) == 0)
            {
                let (offset, character) = characters.next().unwrap();
                boundary = self.caret + offset + character.len_utf8();
            }
        }
        self.caret = boundary;
        if !extend {
            self.anchor = boundary;
        }
    }

    pub fn delete_word(&mut self, backward: bool) {
        if self.selection().is_empty() {
            self.move_word(if backward { -1 } else { 1 }, true);
        }
        self.delete(backward);
    }

    /// Bounded insertion is atomic; a rejected paste preserves text and selection.
    pub fn insert(&mut self, text: &str, max_bytes: usize) -> bool {
        if text.chars().any(char::is_control) {
            return false;
        }
        let selection = self.selection();
        let Some(bytes) = self
            .value
            .len()
            .checked_sub(selection.len())
            .and_then(|n| n.checked_add(text.len()))
        else {
            return false;
        };
        if bytes > max_bytes {
            return false;
        }
        self.value.replace_range(selection.clone(), text);
        self.caret = selection.start + text.len();
        self.anchor = self.caret;
        true
    }

    pub fn delete(&mut self, backward: bool) {
        if self.selection().is_empty() {
            self.move_caret(if backward { -1 } else { 1 }, true);
        }
        let selection = self.selection();
        self.value.replace_range(selection.clone(), "");
        self.caret = selection.start;
        self.anchor = self.caret;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pointer_byte_boundary_preserves_invalid_selection() {
        let mut editor = super::TextEdit::new("aéz".into());
        assert!(editor.place_caret(1, false));
        assert!(editor.place_caret(3, true));
        assert_eq!(editor.selection(), 1..3);
        assert!(!editor.place_caret(2, false));
        assert!(!editor.place_caret(5, true));
        assert_eq!(editor.caret(), 3);
        assert_eq!(editor.selection(), 1..3);
        assert_eq!(editor.value(), "aéz");
    }

    use super::TextEdit;

    #[test]
    fn word_navigation_selection_and_deletion_preserve_utf8_edges() {
        let mut edit = TextEdit::new("Vale  星系.alpha_2".into());
        edit.move_word(-1, false);
        assert_eq!(&edit.value()[edit.caret()..], "alpha_2");
        edit.move_word(-1, true);
        assert_eq!(&edit.value()[edit.selection()], ".");
        edit.delete_word(true);
        assert_eq!(edit.value(), "Vale  星系alpha_2");
        edit.move_edge(false, false);
        edit.move_word(1, false);
        assert_eq!(&edit.value()[..edit.caret()], "Vale  ");
        edit.delete_word(false);
        assert_eq!(edit.value(), "Vale  ");
        edit.delete_word(true);
        assert_eq!(edit.value(), "");
    }

    #[test]
    fn unicode_selection_replacement_and_deletion_preserve_boundaries() {
        let mut edit = TextEdit::new("A星éZ".into());
        edit.move_caret(-1, false);
        edit.move_caret(-1, true);
        assert_eq!(&edit.value()[edit.selection()], "é");
        assert!(edit.insert("🌌", 32));
        assert_eq!(edit.value(), "A星🌌Z");
        edit.delete(true);
        assert_eq!(edit.value(), "A星Z");
        edit.move_edge(false, false);
        edit.delete(false);
        assert_eq!(edit.value(), "星Z");
    }

    #[test]
    fn rejected_paste_keeps_selection_and_select_all_replaces() {
        let mut edit = TextEdit::new("seed".into());
        edit.select_all();
        let before = edit.clone();
        assert!(!edit.insert("too long", 4));
        assert_eq!(edit, before);
        assert!(!edit.insert("bad\n", 8));
        assert_eq!(edit, before);
        assert!(edit.insert("42", 4));
        assert_eq!(edit.value(), "42");
    }
}
