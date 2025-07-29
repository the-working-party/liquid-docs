const { execSync } = require("child_process");
const fs = require("fs");

// Create test file
const test_content = `First line of the test file
Second line with some words
TODO: This is a test task
Another line here
TODO: Another task
Last line of the file`;

fs.writeFileSync("test.txt", test_content);

console.log("Running CLI tests...\n");

// Test commands
const tests = [
	{ cmd: "node index.js first test.txt", expected: "First line" },
	{ cmd: "node index.js stats test.txt", expected: "Lines: 6" },
	{
		cmd: "node index.js count test.txt --pattern TODO",
		expected: "found 2 times",
	},
	{
		cmd: "node index.js lines test.txt --start 1 --end 3",
		expected: "1: Second line",
	},
];

tests.forEach((test) => {
	try {
		const output = execSync(test.cmd, { encoding: "utf8" });
		const passed = output.includes(test.expected);
		console.log(`✓ ${test.cmd}`);
		if (!passed) {
			console.log(`  Expected: ${test.expected}`);
			console.log(`  Got: ${output}`);
		}
	} catch (error) {
		console.log(`✗ ${test.cmd}`);
		console.log(`  Error: ${error.message}`);
	}
});

// Cleanup
fs.unlinkSync("test.txt");
console.log("\nTests complete!");
