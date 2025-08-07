mod twp_types;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use twp_types::TwpTypes;

#[derive(Debug, Serialize)]
pub struct LiquidFile {
	pub path: String,
	pub liquid_types: Vec<DocBlock>,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct DocBlock {
	pub description: String,
	pub param: Vec<Param>,
	pub example: String,
}

#[derive(Debug, Serialize, PartialEq, Default)]
pub enum ParamType {
	#[default]
	String,
	Number,
	Boolean,
	Object,
}

#[derive(Debug, Serialize, PartialEq, Default)]
pub struct Param {
	pub name: String,
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub type_: Option<ParamType>,
	pub optional: bool,
}

#[derive(Debug, Deserialize)]
struct FileInput {
	path: String,
	content: String,
}

#[wasm_bindgen]
pub fn parse(input: JsValue) -> Result<JsValue, JsValue> {
	let files: Vec<FileInput> = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

	let mut all_files = Vec::with_capacity(files.len());

	for file in files {
		if let Some(blocks) = TwpTypes::extract_doc_blocks(&file.content) {
			let mut liquid_file = LiquidFile {
				path: file.path,
				liquid_types: Vec::with_capacity(blocks.len()),
			};

			for block in blocks {
				if let Ok(block_type) = TwpTypes::parse_doc_content(block) {
					liquid_file.liquid_types.push(block_type);
				}
			}

			all_files.push(liquid_file);
		}
	}

	serde_wasm_bindgen::to_value(&all_files).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn help() -> String {
	// TODO: write proper help
	format!(
		r#"
 ▀█▀ █ █ █ █▀█ ▄▄ ▀█▀ █▄█ █▀█ █▀▀ █▀▀
  █  ▀▄▀▄▀ █▀▀     █   █  █▀▀ ██▄ ▄▄█ v{}

A parser for Shopify liquid doc tags
https://shopify.dev/docs/storefronts/themes/tools/liquid-doc

Usage: twp-types <path>
"#,
		env!("CARGO_PKG_VERSION")
	)
}
