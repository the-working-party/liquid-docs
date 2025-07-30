use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
pub struct Files {
	pub path: String,
	pub liquid_types: Vec<DocBlock>,
}

#[derive(Debug, Serialize)]
pub struct DocBlock {
	pub description: String,
	pub param: Vec<Param>,
	pub example: String,
}

#[derive(Debug, Serialize)]
pub enum ParamType {
	String,
	Number,
	Boolean,
	Object,
}

#[derive(Debug, Serialize)]
pub struct Param {
	pub name: String,
	#[serde(rename = "type")]
	pub type_: ParamType,
	pub optional: bool,
	pub description: String,
}

#[derive(Debug, Deserialize)]
struct FileInput {
	path: String,
	content: String,
}

#[wasm_bindgen]
pub fn parse(input: JsValue) -> Result<JsValue, JsValue> {
	let _files: Vec<FileInput> = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

	serde_wasm_bindgen::to_value(&vec!["test"]).map_err(|e| JsValue::from_str(&e.to_string()))
}

pub struct TwpTypes<'a> {
	content: &'a str,
	chars: std::iter::Peekable<std::str::CharIndices<'a>>,
	current_pos: Option<usize>,
}

impl<'a> TwpTypes<'a> {
	fn extract_doc_blocks(content: &'a str) -> Option<Vec<&'a str>> {
		if !content.contains("doc") {
			return None;
		}

		if !content.contains("{%") {
			return None;
		}

		let mut parser = Self {
			content,
			chars: content.char_indices().peekable(),
			current_pos: None,
		};

		let mut blocks = Vec::new();

		while let Some((idx, ch)) = parser.chars.next() {
			parser.current_pos = Some(idx);

			if ch == '{' && parser.chars.peek().map(|(_, c)| *c) == Some('%') {
				parser.chars.next(); // consume '%'
				parser.skip_whitespace();

				if parser.peek_matches("#") {
					parser.skip_to_tag_close();
					continue;
				}

				if parser.peek_matches("raw") {
					parser.skip_to_end_tag("endraw");
					continue;
				}

				if parser.peek_matches("comment") {
					parser.skip_to_end_tag("endcomment");
					continue;
				}

				if parser.peek_matches("doc") {
					parser.consume_chars(3);
					let doc_content_start = parser.find_tag_close()?;
					let doc_content_end = parser.find_tag("enddoc", false)?;
					blocks.push(&content[doc_content_start..doc_content_end]);
				}
			}

			if !parser.remaining_content_has("doc") {
				break;
			}
		}

		(!blocks.is_empty()).then_some(blocks)
	}

	fn remaining_content_has(&self, substr: &str) -> bool {
		if let Some(current) = self.current_pos {
			self.content[current..].contains(substr)
		} else {
			true
		}
	}

	fn skip_whitespace(&mut self) {
		while self.chars.peek().map(|(_, ch)| ch.is_whitespace()).unwrap_or(false) {
			let (idx, _) = self.chars.next().unwrap(); // Safe because peek confirmed
			self.current_pos = Some(idx);
		}
	}

	fn peek_matches(&mut self, word: &str) -> bool {
		self
			.chars
			.peek()
			.and_then(|(start_pos, _)| {
				let end_pos = start_pos + word.len();

				if end_pos <= self.content.len() && &self.content[*start_pos..end_pos] == word {
					if end_pos < self.content.len() {
						let next_byte = self.content.as_bytes()[end_pos];
						Some(!next_byte.is_ascii_alphabetic())
					} else {
						Some(true) // End of content is a valid boundary
					}
				} else {
					Some(false)
				}
			})
			.unwrap_or(false)
	}

	fn consume_chars(&mut self, count: usize) {
		let mut last_idx = self.current_pos;

		for _ in 0..count {
			if let Some((idx, _)) = self.chars.next() {
				last_idx = Some(idx);
			} else {
				break;
			}
		}

		self.current_pos = last_idx;
	}

	/// Return position after %}
	fn find_tag_close(&mut self) -> Option<usize> {
		self.skip_whitespace();
		if let Some((_, ch)) = self.chars.peek() {
			if *ch == '%' {
				self.chars.next(); // consume '%'
				if self.chars.peek().map(|(_, c)| *c) == Some('}') {
					self.chars.next(); // consume '}'
					return self.chars.peek().map(|(pos, _)| *pos).or(Some(self.content.len()));
				}
			}
		}
		None
	}

	fn find_tag(&mut self, tag: &str, return_end: bool) -> Option<usize> {
		while let Some((idx, ch)) = self.chars.next() {
			self.current_pos = Some(idx);

			if ch == '{' && self.chars.peek().map(|(_, c)| *c) == Some('%') {
				let tag_start = idx; // Save position before {%
				self.chars.next(); // consume '%'
				self.skip_whitespace();

				if self.peek_matches(tag) {
					if return_end {
						// Consume the tag and find the closing %}
						self.consume_chars(tag.len());
						return self.find_tag_close();
					} else {
						// Return position before {%
						return Some(tag_start);
					}
				}
			}
		}
		None
	}

	fn skip_to_tag_close(&mut self) {
		while let Some((idx, ch)) = self.chars.next() {
			self.current_pos = Some(idx);
			if ch == '%' && self.chars.peek().map(|(_, c)| *c) == Some('}') {
				if let Some((idx, _)) = self.chars.next() {
					self.current_pos = Some(idx);
				}
				break;
			}
		}
	}

	fn skip_to_end_tag(&mut self, end_tag: &str) {
		while let Some((idx, ch)) = self.chars.next() {
			self.current_pos = Some(idx);

			if ch == '{' && self.chars.peek().map(|(_, c)| *c) == Some('%') {
				self.chars.next(); // consume '%'
				self.skip_whitespace();

				if self.peek_matches(end_tag) {
					self.consume_chars(end_tag.len());
					self.find_tag_close(); // Skip to %}
					return;
				}
			}
		}
	}
}

#[wasm_bindgen]
pub fn help() -> String {
	format!(
		r#"
 ▀█▀ █ █ █ █▀█ ▄▄ ▀█▀ █▄█ █▀█ █▀▀ █▀▀
  █  ▀▄▀▄▀ █▀▀     █   █  █▀▀ ██▄ ▄▄█

A parser for Shopify liquid doc tags
https://shopify.dev/docs/storefronts/themes/tools/liquid-doc

Usage: twp-types <path>
"#
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_doc_blocks() {
		assert_eq!(TwpTypes::extract_doc_blocks("test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}\ntest{% enddoc %}\ntest"), Some(vec!["\ntest"]));
		assert_eq!(
			TwpTypes::extract_doc_blocks("{%       doc  %}  test {%  enddoc         %} test"),
			Some(vec!["  test "])
		);
		assert_eq!(TwpTypes::extract_doc_blocks("{%doc%}test{%enddoc%}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% comment %}{% doc %}test{% enddoc %}{% endcomment %}test"), None);
	}
}
