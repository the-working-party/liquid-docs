mod liquid_docs;
mod shopify_liquid_objects;

use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

pub use liquid_docs::LiquidDocs;

/// The return type for [parse_files]
#[derive(Debug, Serialize, Deserialize, Tsify)]
pub struct LiquidFile {
	pub path: String,
	pub liquid_types: Option<ParseResult>,
}

/// The error type for [parse]
#[derive(Debug, Serialize, Deserialize, Tsify)]
pub struct ParseError {
	line: usize,
	column: usize,
	message: String,
}

/// The return type for [parse]
#[derive(Debug, Serialize, Deserialize, Tsify)]
pub struct ParseResult {
	pub success: Vec<DocBlock>,
	pub errors: Vec<ParseError>,
}

/// The three different things Shopify supports inside doc tags
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Tsify)]
pub struct DocBlock {
	pub description: String,
	pub param: Vec<Param>,
	pub example: Vec<String>,
}

/// The different types a parameter can be
#[derive(Debug, Serialize, Deserialize, PartialEq, Default, Tsify)]
pub enum ParamType {
	#[default]
	String,
	Number,
	Boolean,
	Object,
	ArrayOf(Box<ParamType>),
	Shopify(String),
}

/// Type of param type within doc a tag
#[derive(Debug, Serialize, Deserialize, PartialEq, Default, Tsify)]
pub struct Param {
	pub name: String,
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub type_: Option<ParamType>,
	pub optional: bool,
}

/// Input type for [parse_files]
#[derive(Debug, Serialize, Deserialize, Tsify)]
pub struct FileInput {
	path: String,
	content: String,
}

/// Helper function to parse content of a file
fn parse_content(input: &str) -> ParseResult {
	let mut result = ParseResult {
		success: Vec::new(),
		errors: Vec::new(),
	};

	if let Some(blocks) = LiquidDocs::extract_doc_blocks(input) {
		for block in blocks {
			match LiquidDocs::parse_doc_content(block) {
				Ok(block_type) => result.success.push(block_type),
				Err(error) => {
					result.errors.push(ParseError {
						line: error.get_line(),
						column: error.get_column(),
						message: error.to_string(),
					});
				},
			}
		}
	}

	result
}

/// Parse a Vec<FileInput> and return Vec<LiquidFile>
///
/// Throws a JS error if an item in `files` cannot be deserialized into a [FileInput]
#[wasm_bindgen]
pub fn parse_batch(files: Vec<Ts<FileInput>>) -> Result<Vec<Ts<LiquidFile>>, JsError> {
	let mut all_files = Vec::with_capacity(files.len());

	for file in files {
		let file = file.to_rust()?;
		let parse_result = parse_content(&file.content);
		let liquid_file = LiquidFile {
			path: file.path,
			liquid_types: if parse_result.success.is_empty() && parse_result.errors.is_empty() {
				None
			} else {
				Some(parse_result)
			},
		};
		all_files.push(liquid_file.into_ts()?);
	}

	Ok(all_files)
}

/// Parse a string of Liquid code and return Vec<DocBlock>
#[wasm_bindgen]
pub fn parse(input: String) -> Result<Ts<ParseResult>, JsError> {
	Ok(parse_content(&input).into_ts()?)
}
