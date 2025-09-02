<img src="./assets/logo.png" alt="Liquid Docs Logo" width="180" height="180" align="left">

```
 █   █ █▀█ █ █ █ █▀▄   █▀▄ █▀█ █▀▀ █▀▀
 █▄▄ █ ▀▀█ █▄█ █ █▄▀   █▄▀ █▄█ █▄▄ ▄▄█
```

A parser for [Shopify liquid doc tags](https://shopify.dev/docs/storefronts/themes/tools/liquid-doc)
that allows you to extract the `{% doc %}` content of a liquid file into an object and check liquid files
to make sure they all have a doc block. Written in Rust and compiled to WASM to make it run in node and the browser.

<br>

## Content

- [Goals](#goals)
- [Parser](#parser)
- [Checker](#checker)
- [Contribution](#contribution)
- [License](#license)

## Goals

This project wants to stay as close to, how Shopify interprets the doc tag, as possible.
Right now this library supports only what has been noted in the [Shopify liquid docs](https://shopify.dev/docs/storefronts/themes/tools/liquid-doc):
- `@description`, `@param` and `@example`
- Description without `@description` at the top
- Param types: `string`, `string[]`, `number`, `number[]`, `boolean`, `boolean[]`, `object` and `object[]`
- Param types also supports Shopify objects via the `Shopify` type. e.g. `{ Shopify: "currency" }`
- Param optionality
- Param type and description are optional
- Multiple examples

## Parser

This library can be used as a library in your or JS/TS (or Rust) project.

To install:
```sh
npm i @the-working-party/liquid-docs
```

```ts
import { parse, ParseResult } from "@the-working-party/liquid-docs";

// An example liquid snippet file
const result: ParseResult = parse(`
{%- doc -%}
Renders an image block.

@param {string} [loading] - The html loading attribute
@param {string} alt       - The alt text for the image

@example
{% render 'image',
  loading: 'eager',
%}
{%- enddoc -%}

<image-block
  ratio="{{ block.settings.ratio }}"
  height="{{ block.settings.height }}"
  style="--border-radius: {{ block.settings.border_radius }}px;"
  {{ block.shopify_attributes }}
>
  {{ closest.product.featured_image, loading: loading, alt: alt | default: closest.product.title }}
</image-block>

{% stylesheet %}
...
{% endstylesheet %}

{% schema %}
...
{% endschema %}
`);

console.log(result);
/*
[
  {
    "description": "Renders an image block.",
    "param": [
      {
        "name": "loading",
        "description": "The html loading attribute",
        "type": "String",
        "optional": true
      },
      {
        "name": "alt",
        "description": "The alt text for the image",
        "type": "String",
        "optional": false
      }
    ],
    "example": ["{% render 'image',\n  loading: 'eager',\n%}"]
  }
]
*/
```

## Checker

The checker is a built-in CLI tool that allows you to check every file within a
given glob for the existence of doc tags.<br>
The checker will return a non-zero error code if it finds a file that does not
contain a doc tag.

```sh
$ npm i -g @the-working-party/liquid-docs
```

Usage:
```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid"
Checking files...
✔️ blocks/image_block.liquid
✔️ blocks/cart-drawer.liquid
✔️ snippets/card.liquid

✨ All liquid files (3) have doc tags
```

_(exit code = `0`)_

Or when it fails:
```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid"
Checking files...
✔️ blocks/image_block.liquid
✖️ blocks/cart-drawer.liquid
✔️ snippets/card.liquid

Found 1 liquid file without doc tags
```

_(exit code = `1`)_

### Checker Options

#### Warn
Flag: `-w` | `--warn`<br>
Throw a warning instead of an error on files without doc tags.

```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid" -w
✔️ blocks/image_block.liquid
✖️ blocks/cart-drawer.liquid
✔️ snippets/card.liquid

Found 1 liquid file without doc tags
```

_(exit code = `0`)_

#### Error on Parsing
Flag: `-e` | `--eparse`<br>
Error on parsing issues (default: warning).
Parsing issues: unsupported type, missing parameter name etc

```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid" -e
✔️ blocks/image_block.liquid
✔️ snippets/card.liquid

Parsing errors:
  tests/fixtures/fails/parsin_error.liquid: Unknown parameter type on 4:10: "unknown"

✨ All liquid files (2) have doc tags
```

_(exit code = `1`)_

#### CI Mode
Flag: `-c` | `--ci`<br>
Run the check in CI mode.
This will output a GCC diagnostic format:<br>
`<file>:<line>:<column>: <severity>: <message>`<br>
And a [GitHub annotation format](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#setting-a-warning-message):
`::<severity> file=<path>,line=<line>[,col=<column>]::<message>`

```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid" -c
tests/fixtures/fails/parsin_error.liquid:4:10: warning: Unknown parameter type on 4:10: "unknown"
::warning file=tests/fixtures/fails/parsin_error.liquid,line=4,col=10::Unknown parameter type on 4:10: "unknown"
tests/fixtures/fails/missing_doc.liquid:1:1: error: Missing doc
::error file=tests/fixtures/fails/missing_doc.liquid,line=1,col=1::Missing doc
```

### GitHub Action

In addition to the annotations the checker leaves in CI mode,
you can also add inline comments with actions like reviewdog:

```yml
name: Testing liquid files

on:
  pull_request:

permissions:
  contents: read
  pull-requests: write
  checks: write

jobs:
  liquid-docs:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4

      - name: Install liquid-docs
        run: npm i -g @the-working-party/liquid-docs

      - name: Setup reviewdog
        uses: reviewdog/action-setup@v1
        with:
          reviewdog_version: latest

      - name: Run liquid-docs with reviewdog
        env:
          REVIEWDOG_GITHUB_API_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          liquid-docs-check "{blocks,snippets}/*.liquid" --ci 2>&1 | \
            reviewdog \
              -efm="%-G::%.%#" \
              -efm="%f:%l:%c: %t%*[^:]: %m" \
              -name="liquid-docs" \
              -reporter=github-pr-review \
              -filter-mode=nofilter \
```

### Performance

> [!NOTE]
> The checker will collect files into 10MB batches to make sure we don't hit
> WASM limits while still reducing the hops between WASM and JS to a minimum
> Theme Size     | No Batching    | 10MB Batch Calls
> -------------- | -------------- | ----------------
> 5MB (small)    | 2              | 2
> 10MB (typical) | 2              | 2
> 15MB (large)   | 2              | 4
> 50MB (huge)    | 2 (risky)      | 10 (safe)
> 200MB (extreme)| crashes        | 40 (still works)

## Contribution

To contribute please note:
- As much of the logic as possible is kept in the rust code base to keep this library fast and
efficient. JS is only used to interface with the filesystem as WASI isn't mature enough yet.
- We use the definitions of the upstream [Shopify/theme-liquid-docs](https://raw.githubusercontent.com/Shopify/theme-liquid-docs/main/data/objects.json) repo to detect valid types. This is checked in a [Github action](./.github/workflows/update_shopify_objects.yml) once a day and if changes are found a PR is generated automatically.

## Releases

- v3.2.0  - Converted JavaScript wrapper to TypeScript, fixed small parser bugs
- v3.1.0  - Added CI mode, error on parsing issues and warn flags to checker, Improved errors with line and column number
- v3.0.0  - Extracting legal Shopify objects directly from Shopify codebase, renamed `Unknown` type to `Shopify`
- v2.0.0  - Added support for unknown types, checker does not error on unknown types anymore
- v1.1.0  - Added support of array types in param
- v1.0.2  - Fixed version display in help and version flag
- v1.0.1  - Fixed tarball wasm inclusion
- v1.0.0  - First release

## License

(c) by [The Working Party](https://theworkingparty.com.au/)<br>
License [MIT](./LICENSE)
