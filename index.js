#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const glob = require("glob");

const { parse, parse_batch, TwpTypes } = require("./pkg/liquid_docs.js");

// Process files in buffer-based batches for optimal performance
// 10MB batches balance memory usage and WASM boundary crossing overhead
const MAX_BUFFER_SIZE = 10 * 1024 * 1024;

function* batch_files(file_path, max_buffer_size = MAX_BUFFER_SIZE) {
	const files = glob.sync(file_path, { cwd: process.cwd() });
	let batch = [];
	let current_size = 0;

	for (const file_path of files) {
		const content = fs.readFileSync(
			path.join(process.cwd(), file_path),
			"utf8",
		);
		const file_size = Buffer.byteLength(content, "utf8");

		// If single file exceeds buffer, send it alone
		if (file_size > max_buffer_size && batch.length === 0) {
			yield [{ path: file_path, content }];
			continue;
		}

		// If adding this file would exceed buffer, yield current batch first
		if (current_size + file_size > max_buffer_size && batch.length > 0) {
			yield batch;
			batch = [];
			current_size = 0;
		}

		batch.push({ path: file_path, content });
		current_size += file_size;
	}

	// Yield remaining files in last batch
	if (batch.length > 0) {
		yield batch;
	}
}

function parse_files(file_path, max_buffer_size = MAX_BUFFER_SIZE) {
	const results = [];
	for (const batch of batch_files(file_path, max_buffer_size)) {
		const batch_results = parse_batch(batch);
		results.push(...batch_results);
	}

	return results;
}

module.exports = {
	batch_files,
	parse,
	parse_batch,
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

console.log("Checking files...");
let found_without_types = 0;
let errors = [];
let file_count = 0;

for (const batch of batch_files(file_path, MAX_BUFFER_SIZE)) {
	const batch_results = parse_batch(batch);

	for (const file of batch_results) {
		file_count++;

		if (file.liquid_types) {
			process.stdout.write("✔️");
			if (file.liquid_types.errors?.length > 0) {
				errors.push(`  Errors: ${file.liquid_types.errors}`);
			}
		} else {
			process.stdout.write("\x1B[31m✖️");
			found_without_types++;
		}
		process.stdout.write(` ${file.path}\x1B[39m\n`);
	}
}

if (errors.length > 0) {
	console.warn("\nErrors:");
	errors.forEach((error) => console.warn(error));
}

if (found_without_types > 0) {
	console.log(
		`\nFound ${found_without_types} liquid file${found_without_types > 1 ? "s" : ""} without doc tags`,
	);
	process.exit(1);
} else {
	console.log(`\n✨ All liquid files (${file_count}) have doc tags`);
	process.exit(0);
}
