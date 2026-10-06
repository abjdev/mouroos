use alloc::string::String;
use spin::Mutex;

static CLIPBOARD_TEXT: Mutex<String> = Mutex::new(String::new());

/// Store text into the global system clipboard
pub fn set_text(text: &str) {
    let mut cb = CLIPBOARD_TEXT.lock();
    cb.clear();
    cb.push_str(text);
}

/// Retrieve text from the global system clipboard
pub fn get_text() -> String {
    CLIPBOARD_TEXT.lock().clone()
}

/// Check if the clipboard currently has non-empty text
pub fn has_text() -> bool {
    !CLIPBOARD_TEXT.lock().is_empty()
}

/// Clear the clipboard contents
pub fn clear() {
    CLIPBOARD_TEXT.lock().clear();
}
