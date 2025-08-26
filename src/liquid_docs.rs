use serde::Serialize;

use crate::{DocBlock, Param, ParamType, ParseError, shopify_liquid_objects::SHOPIFY_ALLOWED_OBJECTS};

/// The error types our [LiquidDocs] methods could throw
#[derive(Debug, PartialEq, Serialize)]
pub enum ParsingError {
	MissingParameterName {
		line: usize,
		column: usize,
		message: String,
	},
	// TODO: add line, column and message to MissingOptionalClosingBracket, UnexpectedParameterEnd and UnknownParameterType
	MissingOptionalClosingBracket(String),
	UnexpectedParameterEnd(String),
	UnknownParameterType(String),
	NoDocContentFound,
}

impl std::fmt::Display for ParsingError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ParsingError::MissingParameterName { line, column, message } => {
				write!(f, "Missing parameter on {line}:{column} near this line:\n{message}")
			},
			ParsingError::MissingOptionalClosingBracket(line) => {
				write!(f, "Missing closing bracket for parameter optionality near this line:\n{}", line)
			},
			ParsingError::UnexpectedParameterEnd(line) => write!(f, "Unexpected parameter end near this line:\n {}", line),
			ParsingError::UnknownParameterType(item) => write!(f, "Unknown parameter type: \"{}\"", item),
			ParsingError::NoDocContentFound => write!(f, "No doc content found"),
		}
	}
}

impl From<ParsingError> for ParseError {
	fn from(error: ParsingError) -> Self {
		match error {
			ParsingError::MissingParameterName { line, column, message } => ParseError {
				line,
				column,
				message: format!("Missing parameter at position {}: {}", column, message),
			},
			ParsingError::MissingOptionalClosingBracket(content) => ParseError {
				line: 0,
				column: 0,
				message: format!("Missing closing bracket for parameter optionality: {}", content),
			},
			ParsingError::UnexpectedParameterEnd(content) => ParseError {
				line: 0,
				column: 0,
				message: format!("Unexpected parameter end: {}", content),
			},
			ParsingError::UnknownParameterType(param_type) => ParseError {
				line: 0,
				column: 0,
				message: format!("Unknown parameter type: {}", param_type),
			},
			ParsingError::NoDocContentFound => ParseError {
				line: 0,
				column: 0,
				message: String::from("No documentation content found"),
			},
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
					parser.skip_to_tag("endraw", true);
					continue;
				}

				if parser.peek_matches("comment") {
					parser.skip_to_tag("endcomment", true);
					continue;
				}

				if parser.peek_matches("doc") {
					parser.consume_chars(3);
					let doc_content_start = parser.skip_to_tag_close()?;
					let doc_content_end = parser.skip_to_tag("enddoc", false)?;
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
					let (start_pos, ch) = if let Some((pos, ch)) = parser.chars.peek() {
						(*pos, *ch)
					} else {
						return Err(ParsingError::UnexpectedParameterEnd(String::from(&content[line_start..])));
					};

					// @param type (optional)
					if ch == '{' {
						if parser.chars.next().is_none() {
							return Err(ParsingError::UnexpectedParameterEnd(String::from(&content[line_start..])));
						};

						if let Some(end_pos) = parser.consume_until("}") {
							let mut type_name = content[start_pos + 1..end_pos].trim();
							let is_array = if type_name.ends_with("[]") {
								type_name = &type_name[..type_name.len() - 2];
								true
							} else {
								false
							};

							let explicit_type = if type_name == "string" {
								ParamType::String
							} else if type_name == "number" {
								ParamType::Number
							} else if type_name == "boolean" {
								ParamType::Boolean
							} else if type_name == "object" {
								ParamType::Object
							} else {
								let is_valid_param_type = matches!(type_name, "string" | "number" | "boolean" | "object")
									|| SHOPIFY_ALLOWED_OBJECTS.contains(&type_name);

								if !is_valid_param_type {
									return Err(ParsingError::UnknownParameterType(String::from(type_name)));
								} else {
									ParamType::Shopify(String::from(type_name))
								}
							};

							if is_array {
								param.type_ = Some(ParamType::ArrayOf(Box::new(explicit_type)));
							} else {
								param.type_ = Some(explicit_type);
							}
						} else {
							return Err(ParsingError::UnexpectedParameterEnd(String::from(&content[line_start..])));
						}

						parser.chars.next(); // consume '}'
					}

					// @param optionality
					parser.consume_whitespace_until_newline();
					let (start_pos, optional) = if let Some((pos, ch)) = parser.chars.peek() {
						if ch == &'[' { (*pos + 1, true) } else { (*pos, false) }
					} else {
						let (line, column) = parser.get_line_and_column(line_start);
						return Err(ParsingError::MissingParameterName {
							line,
							column,
							message: String::from(&content[line_start..]),
						});
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
						let (line, column) = parser.get_line_and_column(line_start);
						return Err(ParsingError::MissingParameterName {
							line,
							column,
							message: String::from(&content[line_start..]),
						});
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
	fn peek_matches(&mut self, needle: &str) -> bool {
		self
			.chars
			.peek()
			.map(|(start_pos, _)| {
				let end_pos = start_pos + needle.len();

				if end_pos <= self.content.len() && self.content[*start_pos..end_pos].eq_ignore_ascii_case(needle) {
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
	fn skip_to_tag(&mut self, tag: &str, return_end: bool) -> Option<usize> {
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

	/// Get the line and column (1 indexed) of a given byte offset in the input stream
	fn get_line_and_column(&self, byte_offset: usize) -> (usize, usize) {
		let bytes = self.content.as_bytes();
		let mut line = 1;
		let mut last_newline_pos = 0;

		for (i, byte) in bytes.iter().enumerate().take(byte_offset.min(bytes.len())) {
			if *byte == b'\n' {
				line += 1;
				last_newline_pos = i + 1;
			}
		}

		let column = byte_offset - last_newline_pos + 1;
		(line, column)
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

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param {collection} foo - bar\n\n end\n"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::Shopify(String::from("collection"))),
					optional: false,
				},],
				example: Vec::new(),
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

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param {string[]  } [foo] bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::ArrayOf(Box::new(ParamType::String))),
					optional: true,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param {  number[]} [foo] bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::ArrayOf(Box::new(ParamType::Number))),
					optional: true,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param { boolean[] } foo bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::ArrayOf(Box::new(ParamType::Boolean))),
					optional: false,
				},],
				example: Vec::new()
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n@param {object[]} foo bar"),
			Ok(DocBlock {
				description: String::from("Description with words"),
				param: vec![Param {
					name: String::from("foo"),
					description: Some(String::from("bar")),
					type_: Some(ParamType::ArrayOf(Box::new(ParamType::Object))),
					optional: false,
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
			LiquidDocs::parse_doc_content("Description with words\n @param \n"),
			Err(ParsingError::MissingParameterName {
				line: 2,
				column: 2,
				message: String::from("@param \n")
			})
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n  @param foo\n  @param \n  @param foo"),
			Err(ParsingError::MissingParameterName {
				line: 3,
				column: 3,
				message: String::from("@param \n  @param foo")
			})
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
			Err(ParsingError::UnexpectedParameterEnd(String::from("@param {string foo bar")))
		);

		assert_eq!(
			LiquidDocs::parse_doc_content("Description with words\n @param {unknown} foo - bar\n\n end\n"),
			Err(ParsingError::UnknownParameterType(String::from("unknown")))
		);
	}

	#[test]
	fn consume_whitespace_test() {
		let content = " \n mid    \nend!";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((3, 'm')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((4, 'i')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((5, 'd')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((11, 'e')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((12, 'n')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((13, 'd')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), Some((14, '!')));
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), None);
		instance.consume_whitespace();
		assert_eq!(instance.chars.next(), None);
	}

	#[test]
	fn consume_whitespace_until_newline_test() {
		let content = " \n mid    \nend!";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((1, '\n')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((3, 'm')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((4, 'i')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((5, 'd')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((10, '\n')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((11, 'e')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((12, 'n')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((13, 'd')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), Some((14, '!')));
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), None);
		instance.consume_whitespace_until_newline();
		assert_eq!(instance.chars.next(), None);
	}

	#[test]
	fn skip_dash_test() {
		let content = "{% tag -%}";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((0, '{')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((1, '%')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((2, ' ')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((3, 't')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((4, 'a')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((5, 'g')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((6, ' ')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((8, '%')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), Some((9, '}')));
		instance.skip_dash();
		assert_eq!(instance.chars.next(), None);
		instance.skip_dash();
		assert_eq!(instance.chars.next(), None);
	}

	#[test]
	fn peek_matches_test() {
		let content = "{% liquid";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		assert_eq!(instance.peek_matches("liquid"), false);
		assert_eq!(instance.chars.next(), Some((0, '{')));
		assert_eq!(instance.peek_matches("liquid"), false);
		assert_eq!(instance.chars.next(), Some((1, '%')));
		assert_eq!(instance.peek_matches("liquid"), false);
		assert_eq!(instance.chars.next(), Some((2, ' ')));
		assert_eq!(instance.peek_matches("liquid"), true);
		assert_eq!(instance.chars.next(), Some((3, 'l')));
		assert_eq!(instance.peek_matches("liquid"), false);
		assert_eq!(instance.peek_matches("iquid"), true);
		assert_eq!(instance.peek_matches("iqui"), false);
	}

	#[test]
	fn consume_chars_test() {
		let content = "0123456789end";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		assert_eq!(instance.chars.next(), Some((0, '0')));
		instance.consume_chars(1);
		assert_eq!(instance.chars.next(), Some((2, '2')));
		instance.consume_chars(5);
		assert_eq!(instance.chars.next(), Some((8, '8')));
	}

	#[test]
	fn skip_to_tag_close_test() {
		let content = "{% tag %}end";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		assert_eq!(instance.chars.next(), Some((0, '{')));
		assert_eq!(instance.chars.next(), Some((1, '%')));
		assert_eq!(instance.chars.next(), Some((2, ' ')));
		instance.skip_to_tag_close();
		assert_eq!(instance.chars.next(), Some((3, 't')));
		instance.skip_to_tag_close();
		assert_eq!(instance.chars.next(), Some((4, 'a')));
		instance.skip_to_tag_close();
		assert_eq!(instance.chars.next(), Some((5, 'g')));
		instance.skip_to_tag_close();
		assert_eq!(instance.chars.next(), Some((9, 'e')));
		instance.skip_to_tag_close();
		assert_eq!(instance.chars.next(), Some((10, 'n')));
	}

	#[test]
	fn consume_until_test() {
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

	#[test]
	fn skip_to_tag_test() {
		let content = "{%- tag-%}stuff stuff {%-    endtag  %}";
		let mut instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		assert_eq!(instance.skip_to_tag("tag", false), Some(0));
		instance.chars = content.char_indices().peekable();
		assert_eq!(instance.skip_to_tag("tag", true), Some(10));
		assert_eq!(instance.skip_to_tag("endtag", false), Some(22));
		instance.chars = content.char_indices().peekable();
		assert_eq!(instance.skip_to_tag("endtag", true), Some(39));
	}

	#[test]
	fn get_line_and_column_test() {
		let content = "12345\n678910\n1112131415\n1617181920";
		let instance = LiquidDocs {
			content,
			chars: content.char_indices().peekable(),
		};

		assert_eq!(&instance.content[4..5], "5");
		assert_eq!(instance.get_line_and_column(4), (1, 5));

		assert_eq!(&instance.content[19..21], "14");
		assert_eq!(instance.get_line_and_column(19), (3, 7));
	}
}
