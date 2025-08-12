mod liquid_docs;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub use liquid_docs::LiquidDocs;

/// The return type for [parse_files]
#[derive(Debug, Serialize)]
pub struct LiquidFile {
	pub path: String,
	pub liquid_types: Option<Vec<DocBlock>>,
}

/// The three different things Shopify supports inside doc tags
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct DocBlock {
	pub description: String,
	pub param: Vec<Param>,
	pub example: String,
}

/// The different types a parameter can be
#[derive(Debug, Serialize, PartialEq, Default)]
pub enum ParamType {
	#[default]
	String,
	Number,
	Boolean,
	Object,
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

/// Parse a Vec<FileInput> and return Vec<LiquidFile>
#[wasm_bindgen]
pub fn parse_files(input: JsValue) -> Result<JsValue, JsValue> {
	let files: Vec<FileInput> = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

	let mut all_files = Vec::with_capacity(files.len());

	for file in files {
		if let Some(blocks) = LiquidDocs::extract_doc_blocks(&file.content) {
			let mut liquid_types = Vec::with_capacity(blocks.len());

			for block in blocks {
				if let Ok(block_type) = LiquidDocs::parse_doc_content(block) {
					liquid_types.push(block_type);
				}
			}

			all_files.push(LiquidFile {
				path: file.path,
				liquid_types: Some(liquid_types),
			});
		} else {
			all_files.push(LiquidFile {
				path: file.path,
				liquid_types: None,
			});
		}
	}

	serde_wasm_bindgen::to_value(&all_files).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse a string of Liquid code and return Vec<DocBlock>
#[wasm_bindgen]
pub fn parse(input: String) -> Result<JsValue, JsValue> {
	let mut liquid_types = Vec::new();
	if let Some(blocks) = LiquidDocs::extract_doc_blocks(&input) {
		for block in blocks {
			if let Ok(block_type) = LiquidDocs::parse_doc_content(block) {
				liquid_types.push(block_type);
			}
		}
	}
	serde_wasm_bindgen::to_value(&liquid_types).map_err(|e| JsValue::from_str(&e.to_string()))
}
