const { execSync } = require("child_process");

const { parse } = require("../pkg/liquid_docs.js");

const PARSE_TESTS = [
	{
		title: "Complex example",
		content: `
{%- comment -%}
	Due to accessibility requirements, we are regrouping all logical elements into the <a> element.
	If a screen reader user is tabbing through the page, or browsing by links on the page with duplicated links,
	they will appear identical which may lead to a confusing user experience.
{%- endcomment -%}

{%  doc   %}
@description               - Card snippet to display a card with an image and title
@param {string} [card_class] Optional class on the parent element
@param {number} sizes -      The sizes attribute for the image
@param {boolean[]} foo     - An array of booleans
@param {currency} [bar]    - A currency value
{%enddoc   %}
<a href="{{ article.url }}" class="article-card{% if card_class != blank %} {{ card_class }}{% endif %}">
	{% if article.image %}
		<div class="article-card__image">
			{%- render 'component-image', image: article.image, aspect_ratio: 'natural', max_width: 960, sizes: sizes -%}
			<div class="image-overlay"></div>
		</div>
	{% endif %}

	<div class="article-card__content">
		<h3 class="h5">
			{{ article.title }}
		</h3>

		<div class="paragraph-extra-small article-card__tag">
			{% if article.tags != blank -%}
				<span>
					{{- article.tags.first }}
				</span>
				<div class="separator-dot"></div>
			{% endif %}
			{%- if section.settings.blog_show_date -%}
				{{- article.published_at | date: '%d %B' -}}
			{%- endif %}
		</div>
	</div>
</a>
`,
		expected: [
			{
				description: "Card snippet to display a card with an image and title",
				param: [
					{
						name: "card_class",
						description: "Optional class on the parent element",
						type: "String",
						optional: true,
					},
					{
						name: "sizes",
						description: "The sizes attribute for the image",
						type: "Number",
						optional: false,
					},
					{
						name: "foo",
						description: "An array of booleans",
						type: {
							ArrayOf: "Boolean",
						},
						optional: false,
					},
					{
						name: "bar",
						description: "A currency value",
						type: {
							Shopify: "currency",
						},
						optional: true,
					},
				],
				example: [],
			},
		],
	},
	{
		title: "Multiple docs",
		content: `
{% doc %}
  Description here
  @param {object} images - Some images
{% enddoc %}

<a href="{{ images.url }}">
	<img src="{{ images.url }}" alt="{{ images.alt }}" />
</a>

{% doc %}
  @description          - Second description here
  @param {string}  url  - Link URL
  @param {boolean} open - Open in new tab
{% enddoc %}
<a href="{{ url }}"{% if open %} target="_blank"{% endif %}>Open</a>
`,
		expected: [
			{
				description: "Description here",
				param: [
					{
						name: "images",
						description: "Some images",
						type: "Object",
						optional: false,
					},
				],
				example: [],
			},
			{
				description: "Second description here",
				param: [
					{
						name: "url",
						description: "Link URL",
						type: "String",
						optional: false,
					},
					{
						name: "open",
						description: "Open in new tab",
						type: "Boolean",
						optional: false,
					},
				],
				example: [],
			},
		],
	},
];

console.log("\x1B[4mRUNNING PARSING TESTS\x1B[0m");
let failed = 0;
PARSE_TESTS.forEach((test) => {
	process.stdout.write(`Running test "${test.title}" `);
	let result = parse(test.content);
	if (JSON.stringify(result.success) !== JSON.stringify(test.expected)) {
		process.stdout.write(
			`\x1B[41m FAILED \x1B[49m\n  Expected: ${JSON.stringify(test.expected)}\n  Got:      ${JSON.stringify(result.success)}\n`,
		);
		failed++;
	} else {
		process.stdout.write("\x1B[42m PASSED \x1B[49m\n");
	}
});

if (failed == 0) {
	let passed = PARSE_TESTS.length - failed;
	console.log(
		`\n\x1B[32mPassed ${passed} test${passed > 1 ? "s" : ""}!\x1B[39m`,
	);
} else {
	console.log(
		`\n\x1B[31mFailed ${failed} test${failed > 1 ? "s" : ""}!\x1B[39m`,
	);
	process.exit(1);
}

const CHECK_TESTS = [
	{
		title: "Single folder test",
		path: "tests/fixtures/*.liquid",
		pass: true,
	},
	{
		title: "Multiple folders test with failed passes",
		path: "tests/fixtures/**/*.liquid",
		pass: false,
	},
	{
		title: "Mixed folders test",
		path: "tests/fixtures/{*.liquid,subfolder/**/*.liquid}",
		pass: true,
	},
];

console.log("\n\x1B[4mRUNNING CHECK TESTS\x1B[0m");
failed = 0;
CHECK_TESTS.forEach((test) => {
	process.stdout.write(`Running test "${test.title}" `);
	let passed;
	try {
		execSync(`node ./index.js "${test.path}"`, { encoding: "utf8" });
		passed = true;
	} catch (error) {
		passed = false;
	}

	if (passed == test.pass) {
		process.stdout.write("\x1B[42m PASSED \x1B[49m\n");
	} else {
		process.stdout.write(`\x1B[41m FAILED \x1B[49m\n`);
		failed++;
	}
});

if (failed == 0) {
	let passed = CHECK_TESTS.length - failed;
	console.log(
		`\n\x1B[32mPassed ${passed} test${passed > 1 ? "s" : ""}!\x1B[39m`,
	);
} else {
	console.log(
		`\n\x1B[31mFailed ${failed} test${failed > 1 ? "s" : ""}!\x1B[39m`,
	);
	process.exit(1);
}
