//! Line editor implementation
//! 
//! Provides a line editor with cursor movement, text insertion/deletion,
//! and word-based operations for tab completion.

/// A line editor for command input with cursor support
/// 
/// Manages a text buffer and cursor position, providing methods for:
/// - Text insertion and deletion
/// - Cursor movement
/// - Word-based operations for tab completion
/// 
/// # Example
/// 
/// ```rust
/// use shell::handle::lib::LineEditor;
/// 
/// let mut editor = LineEditor::new();
/// editor.insert('h');
/// editor.insert('i');
/// assert_eq!(editor.buffer, "hi");
/// ```
pub struct LineEditor {
    /// The current text buffer
    pub buffer: String,
    /// Current cursor position (0-based index)
    pub cursor: usize,
}

impl Default for LineEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LineEditor {
    /// Creates a new empty line editor
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    /// Clears the buffer and resets cursor to beginning
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// Inserts a character at the current cursor position
    /// 
    /// The cursor is advanced by one position after insertion.
    /// 
    /// # Arguments
    /// 
    /// * `ch` - The character to insert
    pub fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Deletes the character before the cursor (backspace)
    /// 
    /// Does nothing if the cursor is at the beginning of the line.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
        }
    }

    /// Deletes the character at the cursor position (delete)
    /// 
    /// Does nothing if the cursor is at the end of the line.
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// Moves the cursor one position to the left
    /// 
    /// Does nothing if already at the beginning of the line.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Moves the cursor one position to the right
    /// 
    /// Does nothing if already at the end of the line.
    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
        }
    }

    /// Moves the cursor to the beginning of the line
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end of the line
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Gets the word at the current cursor position
    /// 
    /// Returns the word boundaries and the word text, useful for tab completion.
    /// 
    /// # Returns
    /// 
    /// `Some((start, end, word))` if a word is found, `None` if the buffer is empty
    /// 
    /// Where:
    /// - `start` - Starting index of the word
    /// - `end` - Ending index of the word  
    /// - `word` - The word text as a string slice
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

    /// Replaces a word in the buffer with new text
    /// 
    /// Used for tab completion to replace a partial word with a completed version.
    /// 
    /// # Arguments
    /// 
    /// * `start` - Starting index of the word to replace
    /// * `end` - Ending index of the word to replace
    /// * `replacement` - The new text to insert
    pub fn replace_word(&mut self, start: usize, end: usize, replacement: &str) {
        self.buffer.replace_range(start..end, replacement);
        self.cursor = start + replacement.len();
    }
}
