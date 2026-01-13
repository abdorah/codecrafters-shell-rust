// ============================================
// LINE EDITOR
// ============================================
pub struct LineEditor {
    pub buffer: String,
    pub cursor: usize,
}

impl Default for LineEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LineEditor {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    pub fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn get_word_at_cursor(&self) -> Option<(usize, usize, &str)> {
        if self.buffer.is_empty() {
            return None;
        }

        let bytes = self.buffer.as_bytes();
        let mut start = self.cursor.min(self.buffer.len().saturating_sub(1));
        let mut end = self.cursor;

        while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }

        while end < self.buffer.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }

        if start < end {
            Some((start, end, &self.buffer[start..end]))
        } else {
            None
        }
    }

    pub fn replace_word(&mut self, start: usize, end: usize, replacement: &str) {
        self.buffer.replace_range(start..end, replacement);
        self.cursor = start + replacement.len();
    }
}
