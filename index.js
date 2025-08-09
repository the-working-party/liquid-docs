#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const glob = require("glob");

const { parse_files, help } = require("./pkg/twp_types.js");

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
	const pkg = JSON.parse_files(fs.readFileSync("package.json", "utf8"));
	console.log(`v${pkg.version}`);
	process.exit(0);
}

const files = glob.sync(file_path, { cwd: process.cwd() });
const file_contents = files.map((file_path) => ({
	path: file_path,
	content: fs.readFileSync(path.join(process.cwd(), file_path), "utf8"),
}));

const liquid_types = parse(file_contents);
console.log(JSON.stringify(liquid_types, null, 2));
