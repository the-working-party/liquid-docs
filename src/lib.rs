use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct FileStats {
	pub lines: usize,
	pub words: usize,
	pub chars: usize,
	pub first_line: String,
}

#[wasm_bindgen]
pub fn analyze_file(content: &str) -> Result<JsValue, JsValue> {
	let lines = content.lines().count();
	let words = content.split_whitespace().count();
	let chars = content.chars().count();
	let first_line = content.lines().next().unwrap_or("File is empty").to_string();

	let stats = FileStats {
		lines,
		words,
		chars,
		first_line: String::from(&first_line),
	};

	serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn get_first_line(content: &str) -> String {
	match content.lines().next() {
		Some(line) => String::from(line),
		None => String::from("File is empty"),
	}
}

#[wasm_bindgen]
pub fn count_pattern(content: &str, pattern: &str) -> usize {
	content.matches(pattern).count()
}

#[wasm_bindgen]
pub fn extract_lines(content: &str, start: usize, end: usize) -> Vec<String> {
	content.lines().skip(start).take(end - start).map(String::from).collect::<Vec<String>>()
}

// Log to browser/Node console
#[wasm_bindgen]
pub fn log(msg: &str) {
	web_sys::console::log_1(&msg.into());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn get_first_line_test() {
		assert_eq!(get_first_line("Hello\nWorld"), "Hello");
		assert_eq!(get_first_line(""), "File is empty");
	}

	#[test]
	fn count_pattern_test() {
		assert_eq!(count_pattern("hello hello world", "hello"), 2);
		assert_eq!(count_pattern("rust is great", "python"), 0);
	}
}
