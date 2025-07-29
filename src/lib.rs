use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
pub struct Files {
	pub path: String,
	pub liquid_types: Vec<LiquidType>,
}

#[derive(Debug, Serialize)]
pub struct LiquidType {
	pub description: String,
	pub param: Vec<Param>,
	pub example: String,
}

#[derive(Debug, Serialize)]
pub struct Param {
	pub name: String,
	#[serde(rename = "type")]
	pub type_: String,
	pub description: String,
}

#[derive(Debug, Deserialize)]
struct FileInput {
	path: String,
	content: String,
}

#[wasm_bindgen]
pub struct TwpTypes {}

#[wasm_bindgen]
impl TwpTypes {
	pub fn parse(input: JsValue) -> Result<JsValue, JsValue> {
		let _files: Vec<FileInput> =
			serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;

		log(&format!("{:?}", _files));

		serde_wasm_bindgen::to_value(&vec!["test"]).map_err(|e| JsValue::from_str(&e.to_string()))
	}
}

pub fn log(msg: &str) {
	web_sys::console::log_1(&msg.into());
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
}
