<img src="assets/logo.png" alt="Liquid Docs Logo" width="180" height="180" align="left">

```
 █   █ █▀█ █ █ █ █▀▄   █▀▄ █▀█ █▀▀ █▀▀
 █▄▄ █ ▀▀█ █▄█ █ █▄▀   █▄▀ █▄█ █▄▄ ▄▄█
```

A parser for [Shopify liquid doc tags](https://shopify.dev/docs/storefronts/themes/tools/liquid-doc)
that allows you to extract the `{% doc %}` content of a liquid file into an object, check liquid files
to make sure they all have a doc block and auto generate a doc site for all your liquid files.

<br>

## Content

- [Parser](#parser)
- [Checker](#checker)
- [Builder](#builder)
- [Contribution](#contribution)
- [License](#license)

## Parser

This library can be used as a library in your or JS/TS (or Rust) project.

To install:
```sh
npm i the-working-party/liquid-docs
```

```js
const {
	get_files,   // a helper function to get all files from a glob
	parse,       // parse the input of a single file
	parse_files, // parse a list of files
	TwpTypes,    // the parser struct from Rust
} = require("the-working-party/liquid-docs");

const result = parse(`
{%- doc -%}
Renders an image block.

@param {string} [loading] - The html loading attribute
@param {string} [alt]     - The alt text for the image

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
        "optional": true
      }
    ],
    "example": "{% render 'image',\n  loading: 'eager',\n%}"
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
$ npm i -g the-working-party/liquid-docs
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

Or when it fails:
```sh
$ liquid-docs-check "{blocks,snippets}/*.liquid"
Checking files...
✔️ blocks/image_block.liquid
✖️ blocks/cart-drawer.liquid
✔️ snippets/card.liquid

Found 1 liquid file without doc tags
```

## Builder

TODO

## Contribution

Most of the logic is kept in the rust code base to keep this library fast and
efficient.

## License

(c) by [The Working Party](https://theworkingparty.com.au/)<br>
License [MIT](./LICENSE)
