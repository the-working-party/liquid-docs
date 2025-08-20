mod liquid_docs;
mod shopify_liquid_objects;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub use liquid_docs::LiquidDocs;

/// The return type for [parse_files]
#[derive(Debug, Serialize)]
pub struct LiquidFile {
	pub path: String,
	pub liquid_types: Option<ParseResult>,
}

/// The return type for [parse]
#[derive(Debug, Serialize)]
pub struct ParseResult {
	pub success: Vec<DocBlock>,
	pub errors: Vec<String>,
}

/// The three different things Shopify supports inside doc tags
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct DocBlock {
	pub description: String,
	pub param: Vec<Param>,
	pub example: Vec<String>,
}

/// The different types a parameter can be
#[derive(Debug, Serialize, PartialEq, Default)]
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
#[derive(Debug, Serialize, PartialEq, Default)]
pub struct Param {
	pub name: String,
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub type_: Option<ParamType>,
	pub optional: bool,
}

/// Input type for [parse_files]
#[derive(Debug, Deserialize)]
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
					result.errors.push(error.to_string());
				},
			}
		}
	}

	result
}

/// Parse a Vec<FileInput> and return Vec<LiquidFile>
#[wasm_bindgen]
pub fn parse_batch(input: JsValue) -> Result<JsValue, JsValue> {
	let files: Vec<FileInput> = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

	let mut all_files = Vec::with_capacity(files.len());

	for file in files {
		let parse_result = parse_content(&file.content);
		all_files.push(LiquidFile {
			path: file.path,
			liquid_types: if parse_result.success.is_empty() && parse_result.errors.is_empty() {
				None
			} else {
				Some(parse_result)
			},
		});
	}

	serde_wasm_bindgen::to_value(&all_files).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse a string of Liquid code and return Vec<DocBlock>
#[wasm_bindgen]
pub fn parse(input: String) -> Result<JsValue, JsValue> {
	let result = parse_content(&input);
	serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
