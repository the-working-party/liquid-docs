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

				if end_pos <= self.content.len() && &self.content[*start_pos..end_pos] == word {
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

	/// Find the next given tag in the input stream and either return the position before or after the closing tag
	fn find_tag(&mut self, tag: &str, return_end: bool) -> Option<usize> {
		while let Some((idx, ch)) = self.chars.next() {
			if ch == '{' && self.chars.peek().map(|(_, c)| *c) == Some('%') {
				let tag_start = idx; // Save position before {%
				self.chars.next(); // consume '%'
				self.skip_dash();
				self.skip_whitespace();

				if self.peek_matches(tag) {
					if return_end {
						// Consume the tag and find the closing %}
						self.consume_chars(tag.len());
						return self.skip_to_tag_close();
					} else {
						// Return position before {%
						return Some(tag_start);
					}
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_doc_blocks() {
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
}
