use serde::Serialize;

use crate::{DocBlock, Param, ParamType};

/// The error types our [LiquidDocs] methods could throw
#[derive(Debug, PartialEq, Serialize)]
pub enum ParsingError {
	MissingParameterName(String),
	MissingOptionalClosingBracket(String),
	UnknownParameterType(String),
	UnexpectedParameterEnd(String),
	NoDocContentFound,
}

impl std::fmt::Display for ParsingError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ParsingError::MissingParameterName(line) => write!(f, "Missing parameter near this line:\n{}", line),
			ParsingError::MissingOptionalClosingBracket(line) => {
				write!(f, "Missing closing bracket for parameter optionality near this line:\n{}", line)
			},
			ParsingError::UnknownParameterType(line) => write!(f, "Unknown parameter type near this line:\n{}", line),
			ParsingError::UnexpectedParameterEnd(line) => write!(f, "Unexpected parameter end near this line:\n {}", line),
			ParsingError::NoDocContentFound => write!(f, "No doc content found"),
		}
	}
}

/// The main struct that parses the content of liquid files
pub struct LiquidDocs<'a> {
	content: &'a str,
	chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> LiquidDocs<'a> {
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
				parser.consume_whitespace();

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

	/// Parse doc block content
	pub fn parse_doc_content(content: &'a str) -> Result<DocBlock, ParsingError> {
		let mut parser = Self {
			content,
			chars: content.char_indices().peekable(),
		};

		let mut doc_block = DocBlock::default();

		parser.consume_whitespace();
		while let Some((line_start, ch)) = parser.chars.next() {
			// description without @description
			if doc_block.description.is_empty() && ch != '@' {
				let end_pos = parser.consume_until_either(&["@param ", "@example ", "@description "]).unwrap_or(content.len());
				doc_block.description = String::from(content[line_start..end_pos].trim());
			}

			if ch == '@' {
				// According to specs at https://shopify.dev/docs/storefronts/themes/tools/liquid-doc
				// > If you provide multiple descriptions, then only the first one will appear when hovering over a render tag
				if parser.peek_matches("description") && doc_block.description.is_empty() {
					parser.consume_chars(11);
					parser.consume_whitespace();

					let start_pos = parser.chars.peek().map(|(pos, _)| *pos).unwrap_or(content.len());
					let end_pos =
						parser.consume_until_either(&["@param ", "@example ", "@description "]).unwrap_or(content.len());

					if end_pos > start_pos {
						if let Some(stripped) = content[start_pos..end_pos].trim().strip_prefix('-') {
							doc_block.description = String::from(stripped.trim());
						} else {
							doc_block.description = String::from(content[start_pos..end_pos].trim());
						}
					}
				}

				// @param
				if parser.peek_matches("param") {
					parser.consume_chars(5);
					parser.consume_whitespace_until_newline();
					let mut param = Param::default();
					let (_, ch) = if let Some((pos, ch)) = parser.chars.peek() {
						(*pos, *ch)
					} else {
						return Err(ParsingError::UnexpectedParameterEnd(String::from(&content[line_start..])));
					};

					// @param type (optional)
					if ch == '{' {
						if parser.chars.next().is_none() {
							return Err(ParsingError::UnexpectedParameterEnd(String::from(&content[line_start..])));
						};

						parser.consume_whitespace_until_newline();
						if parser.peek_matches("string") {
							param.type_ = Some(ParamType::String);
							parser.consume_chars(6);
						} else if parser.peek_matches("number") {
							param.type_ = Some(ParamType::Number);
							parser.consume_chars(6);
						} else if parser.peek_matches("boolean") {
							param.type_ = Some(ParamType::Boolean);
							parser.consume_chars(7);
						} else if parser.peek_matches("object") {
							param.type_ = Some(ParamType::Object);
							parser.consume_chars(6);
						} else {
							return Err(ParsingError::UnknownParameterType(String::from(&content[line_start..])));
						}

						parser.consume_until("}");
						parser.chars.next(); // consume '}'
					}

					// @param optionality
					parser.consume_whitespace_until_newline();
					let (start_pos, optional) = if let Some((pos, ch)) = parser.chars.peek() {
						if ch == &'[' { (*pos + 1, true) } else { (*pos, false) }
					} else {
						return Err(ParsingError::MissingParameterName(String::from(&content[line_start..])));
					};
					param.optional = optional;
					if optional {
						parser.chars.next(); // consume '['
					}

					// @param name
					parser.consume_whitespace_until_newline();
					let end_pos = if optional {
						parser
							.consume_until("]")
							.ok_or(ParsingError::MissingOptionalClosingBracket(String::from(&content[line_start..])))?
					} else {
						parser.consume_until_either(&[" ", "\n"]).unwrap_or(content.len())
					};
					param.name = String::from(content[start_pos..end_pos].trim());
					if optional {
						parser.chars.next(); // consume ']'
					}
					if param.name.is_empty() {
						return Err(ParsingError::MissingParameterName(String::from(&content[line_start..])));
					}
					if param.name.contains('\n') {
						return Err(ParsingError::MissingOptionalClosingBracket(String::from(&content[line_start..])));
					}

					// @param description (optional)
					if let Some((_, ch)) = parser.chars.peek()
						&& ch != &'\n'
					{
						parser.consume_whitespace_until_newline();
						let start_pos = if let Some((pos, ch)) = parser.chars.peek() {
							if ch == &'-' { *pos + 1 } else { *pos }
						} else {
							content.len()
						};
						let end_pos = parser.consume_until("\n").unwrap_or(content.len());
						if end_pos > start_pos {
							param.description = Some(String::from(content[start_pos..end_pos].trim()));
						}
					};

					if param != Param::default() {
						doc_block.param.push(param);
					}
				}

				// @example (optional)
				if parser.peek_matches("example") {
					parser.consume_chars(7);
					parser.consume_whitespace_until_newline();
					let start_pos = if let Some((pos, _)) = parser.chars.peek() {
						*pos
					} else {
						content.len()
					};
					let end_pos =
						parser.consume_until_either(&["@param ", "@example ", "@description "]).unwrap_or(content.len());

					let mut example = String::new();
					let indentation_level = &content[start_pos..end_pos].chars().take_while(|c| c.is_whitespace()).count();
					if *indentation_level > 0 {
						content[start_pos..end_pos]
							.trim()
							.lines()
							.map(|line| {
								let chars_to_skip = line.chars().take(*indentation_level - 1).take_while(|c| c.is_whitespace()).count();
								&line[line.char_indices().nth(chars_to_skip).map(|(i, _)| i).unwrap_or(line.len())..]
							})
							.enumerate()
							.for_each(|(idx, stripped_line)| {
								if idx > 0 {
									example.push('\n');
								}
								example.push_str(stripped_line);
							});
					} else {
						example = String::from(content[start_pos..end_pos].trim());
					}

					if !example.is_empty() {
						doc_block.example.push(example);
					}
				}
			}
		}

		if doc_block == DocBlock::default() {
			Err(ParsingError::NoDocContentFound)
		} else {
			Ok(doc_block)
		}
	}

	/// Move the cursor to the next non-whitespace character
	fn consume_whitespace(&mut self) {
		while self.chars.peek().map(|(_, ch)| ch.is_whitespace()).unwrap_or(false) {
			self.chars.next();
		}
	}

	/// Move the cursor to the next non-whitespace character unless it's a newline
	fn consume_whitespace_until_newline(&mut self) {
		while self.chars.peek().map(|(_, ch)| ch.is_whitespace() && ch != &'\n').unwrap_or(false) {
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
		self.consume_whitespace();
		self.skip_dash();
		if let Some((_, ch)) = self.chars.peek()
			&& *ch == '%'
		{
			self.chars.next(); // consume '%'
			if self.chars.peek().map(|(_, c)| *c) == Some('}') {
				self.chars.next(); // consume '}'
				return self.chars.peek().map(|(pos, _)| *pos).or(Some(self.content.len()));
			}
		}
		None
	}

	/// Find the next occurrence of a string and return its position
	fn consume_until(&mut self, target: &str) -> Option<usize> {
		if target.is_empty() {
			return None;
		}

		let first_char = target.chars().next()?;
		let target_len = target.len();

		while let Some((pos, ch)) = self.chars.peek() {
			if *ch == first_char
				&& *pos + target_len <= self.content.len()
				&& self.content[*pos..*pos + target_len] == *target
			{
				return Some(*pos);
			}
			self.chars.next();
		}

		None
	}

	/// Consume until we find the first needle in the list
	fn consume_until_either(&mut self, needles: &[&str]) -> Option<usize> {
		while let Some((pos, _)) = self.chars.peek() {
			let remaining = &self.content[*pos..];

			if needles.iter().any(|&needle| remaining.starts_with(needle)) {
				return Some(*pos);
			}

			self.chars.next();
		}
		None
	}

	/// Find the next given tag in the input stream and either return the position before or after the closing tag
	fn find_tag(&mut self, tag: &str, return_end: bool) -> Option<usize> {
		while let Some(tag_start) = self.consume_until("{%") {
			let current_pos = self.chars.peek().map(|(pos, _)| *pos).unwrap_or(self.content.len());
			self.consume_chars(tag_start - current_pos + 2); // consume chars to tag and tag itself
			let saved_position = tag_start;

			self.skip_dash();
			self.consume_whitespace();

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
				self.consume_whitespace();

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
	fn extract_doc_blocks_test() {
		assert_eq!(LiquidDocs::extract_doc_blocks("test"), None);
		assert_eq!(LiquidDocs::extract_doc_blocks("{% doc %}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{%- doc %}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{%- doc -%}test{% enddoc %}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{%- doc -%}test{%- enddoc %}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{%- doc -%}test{%- enddoc -%}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{% doc %}test{% enddoc1 %}test"), None);
		assert_eq!(
			LiquidDocs::extract_doc_blocks(
				"{% doc %}block1\n  line1\n  line2\n  line3\n\n{% enddoc %}test\n{% doc %}block2{% enddoc %}"
			),
			Some(vec!["block1\n  line1\n  line2\n  line3\n\n", "block2"])
		);
		assert_eq!(LiquidDocs::extract_doc_blocks("{% doc %}\n\ntest{% enddoc %}\n\ntest"), Some(vec!["\n\ntest"]));
		assert_eq!(
			LiquidDocs::extract_doc_blocks("{%       doc  %}  test {%  enddoc         %} test"),
			Some(vec!["  test "])
		);
		assert_eq!(LiquidDocs::extract_doc_blocks("{%doc%}test{%enddoc%}test"), Some(vec!["test"]));
		assert_eq!(LiquidDocs::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(LiquidDocs::extract_doc_blocks("{% raw %}{% doc %}test{% enddoc %}{% endraw %}test"), None);
		assert_eq!(LiquidDocs::extract_doc_blocks("{% comment %}{% doc %}test{% enddoc %}{% endcomment %}test"), None);
		assert_eq!(LiquidDocs::extract_doc_blocks("{% doc %}{% enddoc %}"), Some(vec![""]));

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
		assert_eq!(LiquidDocs::extract_doc_blocks(&content), Some(vec![doc]));

		let doc = r#"
  Provides an example of a snippet description.

  @example
  {% raw %}
    {% render 'example-snippet', title: 'Featured Products', max_items: 3 %}
  {% endraw %}
"#;
		let content = format!(
			r#"{{% doc %}}{doc}{{% enddoc %}}
{{% if article.image %}}
	<div class="article-card__image">
		{{% render 'example-snippet', title: 'Featured Products', max_items: 3 %}}
	</div>
{{% endif %}}
"#
		);
		assert_eq!(LiquidDocs::extract_doc_blocks(&content), Some(vec![doc]));
	}

	#[test]
	fn parse_doc_content_description_test() {
		assert_eq!(
			LiquidDocs::parse_doc_content("test"),
			Ok(DocBlock {
				description: String::from("test"),
				param: Vec::new(),
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
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
			Ok(DocBlock {
				description: String::from("The description 1\n\t\t\tWith new lines\n\t\tand different indentation\nend"),
				param: Vec::new(),
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
@description The description 2
also with new lines
  and some indentation
end
"#
			),
			Ok(DocBlock {
				description: String::from("The description 2\nalso with new lines\n  and some indentation\nend"),
				param: Vec::new(),
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("@description - The description 3"),
			Ok(DocBlock {
				description: String::from("The description 3"),
				param: Vec::new(),
				example: Vec::new()
			})
		);
	}

	#[test]
	fn parse_doc_content_param_complex_test() {
		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
Description with words @ foobar
end!

@param {string}  [var1] - Optional variable 1
		@param {  number  }  var2   - Variable 2
                                  with new line
end
@example
{% render 'example-snippet', var1: 'Featured Products', var2: 3, var5: {} %}
  @param {boolean} [ var3  ]   - Variable 3
  foo @param {object} var5 Variable 5
  @param var6

@example
{% render 'example-snippet',
  var1: variant.price,
  var5: false
%}
"#
			),
			Ok(DocBlock {
				description: String::from("Description with words @ foobar\nend!"),
				param: vec![
					Param {
						name: String::from("var1"),
						description: Some(String::from("Optional variable 1")),
						type_: Some(ParamType::String),
						optional: true,
					},
					Param {
						name: String::from("var2"),
						description: Some(String::from("Variable 2")),
						type_: Some(ParamType::Number),
						optional: false,
					},
					Param {
						name: String::from("var3"),
						description: Some(String::from("Variable 3")),
						type_: Some(ParamType::Boolean),
						optional: true,
					},
					Param {
						name: String::from("var5"),
						description: Some(String::from("Variable 5")),
						type_: Some(ParamType::Object),
						optional: false,
					},
					Param {
						name: String::from("var6"),
						description: None,
						type_: None,
						optional: false,
					},
				],
				example: vec![
					String::from("{% render 'example-snippet', var1: 'Featured Products', var2: 3, var5: {} %}"),
					String::from("{% render 'example-snippet',\n  var1: variant.price,\n  var5: false\n%}")
				]
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
  Intended for use @description foo in a block similar to the button block.
  more lines here
  end

  @param {string} link - link to render
  @example
  {% raw %}
    {% render 'button', link: '@/collections/all' %}
    sadsad @param asdasd
  {% endraw %}

  test @param { object    } [     block] - The block @param things and what not
  @param [foo]

  @description testing

  @example
  {% render 'button', link: '/collections/all' %}
"#
			),
			Ok(DocBlock {
				description: String::from("Intended for use"),
				param: vec![
					Param {
						name: String::from("link"),
						description: Some(String::from("link to render")),
						type_: Some(ParamType::String),
						optional: false,
					},
					Param {
						name: String::from("asdasd"),
						description: None,
						type_: None,
						optional: false,
					},
					Param {
						name: String::from("block"),
						description: Some(String::from("The block @param things and what not")),
						type_: Some(ParamType::Object),
						optional: true,
					},
					Param {
						name: String::from("foo"),
						description: None,
						type_: None,
						optional: true,
					},
				],
				example: vec![
					String::from("{% raw %}\n  {% render 'button', link: '@/collections/all' %}\n  sadsad"),
					String::from("{% render 'button', link: '/collections/all' %}")
				]
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
Intended for use @ description foo in a block similar to the button block.
  more lines here
  end

  @param {string} link - link to render
  @example
  {% raw %}
    {% render 'button', link: '@/collections/all' %}
    sadsad @ param asdasd
  {% endraw %}

  test @param { object    } [     block] - The block @param things and what not
  @param [foo]

  @description testing

  @example
  {% render 'button', link: '/collections/all' %}
"#
			),
			Ok(DocBlock {
				description: String::from(
					"Intended for use @ description foo in a block similar to the button block.\n  more lines here\n  end"
				),
				param: vec![
					Param {
						name: String::from("link"),
						description: Some(String::from("link to render")),
						type_: Some(ParamType::String),
						optional: false,
					},
					Param {
						name: String::from("block"),
						description: Some(String::from("The block @param things and what not")),
						type_: Some(ParamType::Object),
						optional: true,
					},
					Param {
						name: String::from("foo"),
						description: None,
						type_: None,
						optional: true,
					},
				],
				example: vec![
					String::from(
						"{% raw %}\n  {% render 'button', link: '@/collections/all' %}\n  sadsad @ param asdasd\n{% endraw %}\n\ntest"
					),
					String::from("{% render 'button', link: '/collections/all' %}")
				]
			})
		);
	}

	#[test]
	fn parse_doc_content_param_param_test() {
		assert_eq!(
			LiquidDocs::parse_doc_content("@param foo"),
			Ok(DocBlock {
				description: String::new(),
				param: vec![Param {
					name: String::from("foo"),
					description: None,
					type_: None,
					optional: false,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param foo bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: None,
					optional: false,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param {string} foo bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::String),
					optional: false,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param {string} [foo] bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::String),
					optional: true,
				},],
				example: Vec::new()
			})
		);
	}

	#[test]
	fn parse_doc_content_example_indentation_test() {
		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
@example
{% raw %}
	{% render 'card' %}
{% endraw %}
"#
			),
			Ok(DocBlock {
				description: String::new(),
				param: Vec::new(),
				example: vec![String::from("{% raw %}\n\t{% render 'card' %}\n{% endraw %}")],
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
				@example
				{% raw %}
					{% render 'card' %}
				{% endraw %}
				"#
			),
			Ok(DocBlock {
				description: String::new(),
				param: Vec::new(),
				example: vec![String::from("{% raw %}\n\t{% render 'card' %}\n{% endraw %}")],
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
				@example
					{% raw %}
						{% render 'card' %}
					{% endraw %}
				"#
			),
			Ok(DocBlock {
				description: String::new(),
				param: Vec::new(),
				example: vec![String::from("{% raw %}\n\t{% render 'card' %}\n{% endraw %}")],
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content(
				r#"
				@example
					{% raw %}
			{% render 'card' %}
	{% endraw %}
				"#
			),
			Ok(DocBlock {
				description: String::new(),
				param: Vec::new(),
				example: vec![String::from("{% raw %}\n{% render 'card' %}\n{% endraw %}")],
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("@example\n{% raw %}\n{% render 'card' %}\n{% endraw %}"),
			Ok(DocBlock {
				description: String::new(),
				param: Vec::new(),
				example: vec![String::from("{% raw %}\n{% render 'card' %}\n{% endraw %}")],
			})
		);
	}

	#[test]
	fn parse_doc_content_param_error_test() {
		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param {unknown} foo - bar\n\n end\n"),
			Err(ParsingError::UnknownParameterType(String::from("@param {unknown} foo - bar\n\n end\n")))
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param \n"),
			Err(ParsingError::MissingParameterName(String::from("@param \n")))
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param \n @param foo"),
			Err(ParsingError::MissingParameterName(String::from("@param \n @param foo")))
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param "),
			Err(ParsingError::UnexpectedParameterEnd(String::from("@param ")))
		);

		assert_eq!(LiquidDocs::parse_doc_content(""), Err(ParsingError::NoDocContentFound));

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param [foo bar"),
			Err(ParsingError::MissingOptionalClosingBracket(String::from("@param [foo bar")))
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param {string foo bar"),
			Err(ParsingError::MissingParameterName(String::from("@param {string foo bar")))
		);
	}

	#[test]
	fn find_next_test() {
		let content = "start @test end";

		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until("@test"),
			Some(6)
		);
		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until("@"),
			Some(6)
		);
		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until("t"),
			Some(1)
		);
		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until("te"),
			Some(7)
		);
	}

	#[test]
	fn consume_until_either_test() {
		let content = "start @param end";
		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until_either(&["@param ", "@example ", "@description "]),
			Some(6)
		);

		let content = r#"
Description with words @ foobar
end!

@param {string}  [var1] - Optional variable 1"#;
		assert_eq!(
			LiquidDocs {
				content,
				chars: content.char_indices().peekable(),
			}
			.consume_until_either(&["@param ", "@example ", "@description "]),
			Some(39)
		);
	}
}
