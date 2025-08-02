use crate::{DocBlock, Param, ParamType};

/// The main struct that parses the content of liquid files
pub struct TwpTypes<'a> {
	content: &'a str,
	chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> TwpTypes<'a> {
	/// Extract a collection of all doc blocks from the given content without the wrapping doc tag
	pub fn extract_doc_blocks(content: &'a str) -> Option<Vec<&'a str>> {
		// This may find more than just the closing tags for our doc blocks which means we sometimes may not return early
		// but that's still better then never returning early
		let possible_doc_blocks = content.matches("enddoc").count();

		if possible_doc_blocks == 0 {
			return None;
		}

		let mut parser = Self {
			content,
			chars: content.char_indices().peekable(),
		};

		let mut blocks = Vec::with_capacity(possible_doc_blocks);
		let mut found_blocks = 0;

		while let Some((_, ch)) = parser.chars.next() {
			if ch == '{' && parser.chars.peek().map(|(_, c)| *c) == Some('%') {
				parser.chars.next(); // consume '%'
				parser.skip_dash();
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
					let doc_content_start = parser.skip_to_tag_close()?;
					let doc_content_end = parser.find_tag("enddoc", false)?;
					blocks.push(&content[doc_content_start..doc_content_end]);
					found_blocks += 1;
				}
			}

			if found_blocks == possible_doc_blocks {
				break;
			}
		}

		(!blocks.is_empty()).then_some(blocks)
	}

	pub fn parse_doc_content(content: &'a str) -> Option<DocBlock> {
		let mut parser = Self {
			content,
			chars: content.char_indices().peekable(),
		};

		let mut doc_block = DocBlock::default();
		let mut description = String::new();

		while let Some((idx, ch)) = parser.chars.next() {
			if doc_block.description.is_empty() && ch != '@' {
				description.push(ch);
			}

			if doc_block.description.is_empty() && ch == '@' && !description.is_empty() {
				let mut taken = std::mem::take(&mut description);
				Self::trim_in_place(&mut taken);
				doc_block.description = taken;
			}

			if ch == '@' {
				// According to specs at https://shopify.dev/docs/storefronts/themes/tools/liquid-doc
				// > If you provide multiple descriptions, then only the first one will appear when hovering over a render tag
				if parser.peek_matches("description") && doc_block.description.is_empty() {
					parser.consume_chars(12);
					let start_pos = idx + 12;
					let end_pos = parser.find_next("@").unwrap_or(content.len());
					let mut description = content[start_pos..end_pos].to_string();
					Self::trim_in_place(&mut description);
					doc_block.description = description;
					parser.consume_chars(end_pos - start_pos);
				}

				if parser.peek_matches("param") {
					parser.consume_chars(6);
					parser.skip_whitespace();
					let mut param = Param::default();
					let (_, ch) = if let Some((pos, ch)) = parser.chars.peek() {
						(*pos, *ch)
					} else {
						continue;
					};

					let mut is_valid = true;

					if ch == '{' {
						parser.chars.next();
						parser.skip_whitespace();
						if parser.peek_matches("string") {
							param.type_ = ParamType::String;
						} else if parser.peek_matches("number") {
							param.type_ = ParamType::Number;
						} else if parser.peek_matches("boolean") {
							param.type_ = ParamType::Boolean;
						} else if parser.peek_matches("object") {
							param.type_ = ParamType::Object;
						} else {
							parser.find_next("@").unwrap_or(content.len());
							is_valid = false;
						}
					}

					parser.skip_whitespace();
					// TODO: parse name, check optionality, parse description

					if is_valid {
						doc_block.param.push(param);
					}
				}
			}
		}

		if doc_block == DocBlock::default() {
			None
		} else {
			Some(doc_block)
		}
	}

	/// Move the cursor to the next non-whitespace character
	fn skip_whitespace(&mut self) {
		while self.chars.peek().map(|(_, ch)| ch.is_whitespace()).unwrap_or(false) {
			self.chars.next();
		}
	}

	/// Skip an optional dash character for whitespace control
	fn skip_dash(&mut self) {
		if self.chars.peek().map(|(_, ch)| *ch == '-').unwrap_or(false) {
			self.chars.next();
		}
	}

	/// Check if the following content matches a specific substring
	fn peek_matches(&mut self, word: &str) -> bool {
		self
			.chars
			.peek()
			.map(|(start_pos, _)| {
				let end_pos = start_pos + word.len();

				if end_pos <= self.content.len() && self.content[*start_pos..end_pos].eq_ignore_ascii_case(word) {
					if end_pos < self.content.len() {
						// Safe because if the string comparison succeeds, end_pos must be on a char boundary
						let next_byte = self.content.as_bytes()[end_pos];
						!next_byte.is_ascii_alphanumeric()
					} else {
						true // End of content is a valid boundary
					}
				} else {
					false
				}
			})
			.unwrap_or(false)
	}

	/// Consume a number of characters from the input stream
	fn consume_chars(&mut self, count: usize) {
		for _ in 0..count {
			if self.chars.next().is_none() {
				break;
			}
		}
	}

	/// Move to position after next %}
	fn skip_to_tag_close(&mut self) -> Option<usize> {
		self.skip_whitespace();
		self.skip_dash();
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

	/// Find the next occurrence of a string and return its position
	fn find_next(&mut self, target: &str) -> Option<usize> {
		if target.is_empty() {
			return None;
		}

		while let Some((pos, _)) = self.chars.peek() {
			if pos + target.len() <= self.content.len() && self.content[*pos..pos + target.len()] == *target {
				return Some(*pos);
			}
			self.chars.next();
		}

		None
	}

	/// Find the next given tag in the input stream and either return the position before or after the closing tag
	fn find_tag(&mut self, tag: &str, return_end: bool) -> Option<usize> {
		while let Some(tag_start) = self.find_next("{%") {
			let current_pos = self.chars.peek().map(|(pos, _)| *pos).unwrap_or(self.content.len());
			self.consume_chars(tag_start - current_pos + 2); // consume chars to tag and tag itself
			let saved_position = tag_start;

			self.skip_dash();
			self.skip_whitespace();

			if self.peek_matches(tag) {
				if return_end {
					self.consume_chars(tag.len());
					return self.skip_to_tag_close();
				} else {
					return Some(saved_position);
				}
			}
		}

		None
	}

	/// Move to next given closing tag
	fn skip_to_end_tag(&mut self, end_tag: &str) {
		while let Some((_, ch)) = self.chars.next() {
			if ch == '{' && self.chars.peek().map(|(_, c)| *c) == Some('%') {
				self.chars.next(); // consume '%'
				self.skip_dash();
				self.skip_whitespace();

				if self.peek_matches(end_tag) {
					self.consume_chars(end_tag.len());
					self.skip_to_tag_close(); // Skip to %}
					return;
				}
			}
		}
	}

	/// Trim leading and trailing whitespace in place without any extra heap allocation
	fn trim_in_place(s: &mut String) {
		// Leading whitespace
		if let Some(first_non_ws) = s.find(|c: char| !c.is_whitespace()) {
			if first_non_ws > 0 {
				s.drain(..first_non_ws);
			}
		} else {
			s.clear();
			return;
		}

		// Trailing whitespace
		if let Some(last_non_ws) = s.rfind(|c: char| !c.is_whitespace()) {
			s.truncate(last_non_ws + 1);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extract_doc_blocks_test() {
		assert_eq!(TwpTypes::extract_doc_blocks("test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{%- doc %}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{%- doc -%}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{%- doc -%}test{%- enddoc %}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{%- doc -%}test{%- enddoc -%}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}test{% enddoc1 %}test"), None);
		assert_eq!(
			TwpTypes::extract_doc_blocks(
				"{% doc %}block1\n  line1\n  line2\n  line3\n\n{% enddoc %}test\n{% doc %}block2{% enddoc %}"
			),
			Some(vec!["block1\n  line1\n  line2\n  line3\n\n", "block2"])
		);
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}\n\ntest{% enddoc %}\n\ntest"), Some(vec!["\n\ntest"]));
		assert_eq!(
			TwpTypes::extract_doc_blocks("{%       doc  %}  test {%  enddoc         %} test"),
			Some(vec!["  test "])
		);
		assert_eq!(TwpTypes::extract_doc_blocks("{%doc%}test{%enddoc%}test"), Some(vec!["test"]));
		assert_eq!(TwpTypes::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% comment %}{% doc %}test{% enddoc %}{% endcomment %}test"), None);
		assert_eq!(TwpTypes::extract_doc_blocks("{% doc %}{% enddoc %}"), Some(vec![""]));

		let doc = r#"
  Provides an example of a snippet description.

  @param {string} title - The title to display
  @param {number} [max_items] - Optional maximum number of items to show

  @example
  {% render 'example-snippet', title: 'Featured Products', max_items: 3 %}
"#;
		let content = format!(
			r#"{{% doc %}}{doc}{{% enddoc %}}
{{% if article.image %}}
	<div class="article-card__image">
		{{%- render 'component-image', image: article.image, aspect_ratio: 'natural', max_width: 960, sizes: sizes -%}}
		<div class="image-overlay"></div>
	</div>
{{% endif %}}
"#
		);

		assert_eq!(TwpTypes::extract_doc_blocks(&content), Some(vec![doc]));
	}

	#[test]
	fn parse_doc_content_description_test() {
		assert_eq!(TwpTypes::parse_doc_content("test"), None);
		assert_eq!(
			TwpTypes::parse_doc_content(
				r#"
			The description 1
			With new lines
		and different indentation
end

@description The description 2
also with new lines
  and some indentation
end
"#
			),
			Some(DocBlock {
				description: String::from("The description 1\n\t\t\tWith new lines\n\t\tand different indentation\nend"),
				param: Vec::new(),
				example: String::new()
			})
		);
		assert_eq!(
			TwpTypes::parse_doc_content(
				r#"
@description The description 2
also with new lines
  and some indentation
end
"#
			),
			Some(DocBlock {
				description: String::from("The description 2\nalso with new lines\n  and some indentation\nend"),
				param: Vec::new(),
				example: String::new()
			})
		);
	}

	#[test]
	fn parse_doc_content_param_test() {
		assert_eq!(
			TwpTypes::parse_doc_content(
				r#"
Description with words

@param {string}  [var1] - Optional variable 1
@param {number}  var2   - Variable 2
@param {boolean} var3   - Variable 3
@param {unknown} var4   - Variable 4
@param {object}  var5   - Variable 5
"#
			),
			Some(DocBlock {
				description: String::from("Description with words"),
				param: vec![
					Param {
						name: String::from("var1"),
						description: String::from("Optional variable 1"),
						type_: ParamType::String,
						optional: true,
					},
					Param {
						name: String::from("var2"),
						description: String::from("Variable 2"),
						type_: ParamType::Number,
						optional: false,
					},
					Param {
						name: String::from("var3"),
						description: String::from("Variable 3"),
						type_: ParamType::Boolean,
						optional: false,
					},
					Param {
						name: String::from("var4"),
						description: String::from("Variable 4"),
						type_: ParamType::Object,
						optional: false,
					}
				],
				example: String::new()
			})
		);
	}

	#[test]
	fn find_next_test() {
		let content = "start @test end";

		assert_eq!(
			TwpTypes {
				content,
				chars: content.char_indices().peekable(),
			}
			.find_next("@test"),
			Some(6)
		);
		assert_eq!(
			TwpTypes {
				content,
				chars: content.char_indices().peekable(),
			}
			.find_next("@"),
			Some(6)
		);
		assert_eq!(
			TwpTypes {
				content,
				chars: content.char_indices().peekable(),
			}
			.find_next("t"),
			Some(1)
		);
		assert_eq!(
			TwpTypes {
				content,
				chars: content.char_indices().peekable(),
			}
			.find_next("te"),
			Some(7)
		);
	}
}
