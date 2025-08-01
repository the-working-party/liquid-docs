mod twp_types;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use twp_types::TwpTypes;

#[derive(Debug, Serialize)]
pub struct LiquidFile {
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
	let files: Vec<FileInput> = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

	for file in files {
		if let Some(blocks) = TwpTypes::extract_doc_blocks(&file.content) {
			let mut liquid_file = LiquidFile {
				path: file.path,
				liquid_types: Vec::with_capacity(blocks.len()),
			};

			// TODO: Implement parsing logic here
		}
	}

	serde_wasm_bindgen::to_value(&vec!["test"]).map_err(|e| JsValue::from_str(&e.to_string()))
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
