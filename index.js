#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const glob = require("glob");

const { parse, parse_files, TwpTypes } = require("./pkg/twp_types.js");

function get_files(file_path) {
	const files = glob.sync(file_path, { cwd: process.cwd() });
	const file_contents = files.map((file_path) => ({
		path: file_path,
		content: fs.readFileSync(path.join(process.cwd(), file_path), "utf8"),
	}));

	return file_contents;
}

module.exports = {
	get_files,
	parse,
	parse_files,
	TwpTypes,
};

function help() {
	const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
	console.log(`
 █   █ █▀█ █ █ █ █▀▄   █▀▄ █▀█ █▀▀ █▀▀
 █▄▄ █ ▀▀█ █▄█ █ █▄▀   █▄▀ █▄█ █▄▄ ▄▄█ v${pkg.version}

A parser for Shopify liquid doc tags
https://shopify.dev/docs/storefronts/themes/tools/liquid-doc

Usage:
  liquid-docs-check <path>

Description:
  Tests all Shopify Liquid files for the existence of {% doc %} tags.

Arguments:
  <path>         Path to a file or directory containing .liquid files.
                 Can use glob patterns.

Options:
  -h, --help     Show this help message and exit.
  -v, --version  Show version information and exit.

Examples:
  liquid-docs-check "{blocks,snippets}/*.liquid"
  liquid-docs-check "path/to/snippets/*.liquid"
`);
}

const args = process.argv;
const file_path = args[2] || "./*.liquid";

if (!file_path || args.includes("-h") || args.includes("--help")) {
	help();

	if (!file_path) {
		process.exit(1);
	} else {
		process.exit(0);
	}
}

if (args.includes("-v") || args.includes("-V") || args.includes("--version")) {
	const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
	console.log(`v${pkg.version}`);
	process.exit(0);
}

let found_without_types = 0;
console.log("Checking files...");
let data = get_files(file_path);
parse_files(data).forEach((file) => {
	if (file.liquid_types) {
		process.stdout.write("✔️");
	} else {
		process.stdout.write("\x1B[31m✖️");
		found_without_types++;
	}
	process.stdout.write(` ${file.path}\x1B[39m\n`);
});

if (found_without_types > 0) {
	console.log(
		`\nFound ${found_without_types} liquid file${found_without_types > 1 ? "s" : ""} without doc tags`,
	);
	process.exit(1);
} else {
	console.log(`\n✨ All liquid files (${data.length}) have doc tags`);
	process.exit(0);
}
