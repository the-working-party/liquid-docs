#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const {
	analyze_file,
	get_first_line,
	count_pattern,
	extract_lines,
	log,
} = require("./pkg/twp_types.js");

const args = process.argv.slice(2);
const command = args[0];
const file_path = args[1];

function print_usage() {
	console.log(`
Usage: twp-types <command> <file> [options]

Commands:
  first     Get the first line of a file
  stats     Show file statistics
  count     Count pattern occurrences (requires --pattern)
  lines     Extract lines (requires --start and --end)

Examples:
  twp-types first myfile.txt
  twp-types stats myfile.txt
  twp-types count myfile.txt --pattern "TODO"
  twp-types lines myfile.txt --start 10 --end 20
    `);
}

if (!command || !file_path) {
	print_usage();
	process.exit(1);
}

let content;
try {
	content = fs.readFileSync(file_path, "utf8");
} catch (error) {
	console.error(`Error reading file: ${error.message}`);
	process.exit(1);
}

try {
	switch (command) {
		case "first":
			console.log(get_first_line(content));
			break;

		case "stats":
			const stats = analyze_file(content);
			console.log("File Statistics:");
			console.log(`Lines: ${stats.lines}`);
			console.log(`Words: ${stats.words}`);
			console.log(`Characters: ${stats.chars}`);
			console.log(`First line: "${stats.first_line}"`);
			break;

		case "count":
			const pattern_index = args.indexOf("--pattern");
			if (pattern_index === -1 || !args[pattern_index + 1]) {
				console.error("Error: --pattern required for count command");
				process.exit(1);
			}
			const pattern = args[pattern_index + 1];
			const count = count_pattern(content, pattern);
			console.log(`Pattern "${pattern}" found ${count} times`);
			break;

		case "lines":
			const start_index = args.indexOf("--start");
			const end_index = args.indexOf("--end");

			if (start_index === -1 || end_index === -1) {
				console.error("Error: --start and --end required for lines command");
				process.exit(1);
			}

			const start = parseInt(args[start_index + 1]);
			const end = parseInt(args[end_index + 1]);

			const lines = extract_lines(content, start, end);
			lines.forEach((line, i) => {
				console.log(`${start + i}: ${line}`);
			});
			break;

		default:
			console.error(`Unknown command: ${command}`);
			print_usage();
			process.exit(1);
	}
} catch (error) {
	console.error(`Error: ${error.message}`);
	process.exit(1);
}
