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
	const pkg = JSON.parse(
		fs.readFileSync(path.join(__dirname, "package.json"), "utf8"),
	);
	console.log(`
 █   █ █▀█ █ █ █ █▀▄   █▀▄ █▀█ █▀▀ █▀▀
 █▄▄ █ ▀▀█ █▄█ █ █▄▀   █▄▀ █▄█ █▄▄ ▄▄█ v${pkg.version}

A checker for Shopify liquid doc tags
https://shopify.dev/docs/storefronts/themes/tools/liquid-doc

Usage:
  liquid-docs-check <path>

Description:
  Tests all Shopify Liquid files for the existence of {% doc %} tags.

Arguments:
  <path>         Path to a file or directory containing .liquid files.
                 Can use glob patterns.

Options:
  -w, --warn     Throw a warning instead of an error on files without doc tags.
  -e, --eparse   Error on parsing issues.
                 Example: unsupported type, missing parameter name etc
  -c, --ci       Run the check in CI mode.
                 Output uses GCC diagnostic format for CI annotations:
                 <file>:<line>:<column>: <severity>: <message>
                 Example: template.liquid:1:1: error: Missing doc
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
	const pkg = JSON.parse(
		fs.readFileSync(path.join(__dirname, "package.json"), "utf8"),
	);
	console.log(`v${pkg.version}`);
	process.exit(0);
}

const CI_MODE = args.includes("-c") || args.includes("--ci");
const WARNING_MODE = args.includes("-w") || args.includes("--warn");
const ERROR_ON_PARSE_ISSUES = args.includes("-e") || args.includes("--eparse");

if (!CI_MODE) {
	console.log("Checking files...");
}
let found_without_types = 0;
let errors = [];
let file_count = 0;

for (const batch of batch_files(file_path, MAX_BUFFER_SIZE)) {
	const batch_results = parse_batch(batch);

	for (const file of batch_results) {
		file_count++;

		if (file.liquid_types) {
			if (!CI_MODE) {
				process.stdout.write("✔️");
			}

			file.liquid_types.errors.forEach(({ line, column, message }) => {
				if (CI_MODE) {
					errors.push(`${file.path}:${line}:${column}: warning: ${message}`);
				} else {
					errors.push(`  \x1B[31m${file.path}\x1B[39m: ${message}`);
				}
			});
		} else {
			if (!CI_MODE) {
				process.stdout.write("\x1B[31m✖️");
			} else {
				let throw_type = WARNING_MODE ? "warning:" : "error:";
				process.stdout.write(`${file.path}:1:1: ${throw_type} Missing doc\n`);
			}
			found_without_types++;
		}
		if (!CI_MODE) {
			process.stdout.write(` ${file.path}\x1B[39m\n`);
		}
	}
}

if (errors.length > 0) {
	if (!CI_MODE)
		console.warn(`\nParsing ${ERROR_ON_PARSE_ISSUES ? "errors" : "warnings"}:`);
	errors.forEach((error) => console.error(error));
}

if (found_without_types > 0) {
	if (!CI_MODE) {
		console.log(
			`\nFound ${found_without_types} liquid file${found_without_types > 1 ? "s" : ""} without doc tags`,
		);
	}
} else {
	if (!CI_MODE) {
		console.log(`\n✨ All liquid files (${file_count}) have doc tags`);
	}
}

if (
	(found_without_types > 0 && WARNING_MODE) ||
	(found_without_types === 0 && !ERROR_ON_PARSE_ISSUES)
) {
	process.exit(0);
} else {
	process.exit(1);
}
