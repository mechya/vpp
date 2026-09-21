# VPP Compiler — Viewer Package Platform

> **Reference specification.** This page describes how the C++ compiler behaved before it was removed (last present in commit `7cf865e`; read its source with `git show 7cf865e:compiler/src/main.cpp`). The Rust port in `crates/vpp-compiler` reproduces this behaviour unless `docs/rust-port.md` says otherwise. Commands use the tool names the Rust port keeps (`vppc`, `vpppack`, `vpp-viewer`); they work once that crate is ported.

**VPP Compiler** is the source-code compilation component of **VPP — Viewer Package Platform**.

It turns a site's pages, layouts, includes, components, stylesheets, and scripts into the binary resources the VPP Runtime executes, one set per page, so that release pages ship no source text.

## Current State

The compiler is the `vppc` tool, built into `vppc`.

### Compile a site

```text
vppc examples\hello-world
```

A site is a directory with `vpp.json` and `pages\*.html`. A directory with only an `index.html` is a one-page site whose page is called `index`. Each page is expanded with its layout, includes, and components, and written as:

```text
examples\hello-world\dist\
├── home\
│   ├── dom.bin                          the finished page
│   ├── style\00-global.bin              one resource per stylesheet, in load order
│   ├── style\01-component-stat-card.bin
│   ├── code\00-app.bin                  one resource per script, in load order
│   └── code\01-home.bin
└── about\
    └── ...
```

One resource per source file means two pages that use the same stylesheet or script produce identical resources, and the packager and viewer share them by hash.

`vppc pages\home.html` compiles a single page into the same layout. `vppc app.js` checks one script and writes it as `code.bin` next to it, or to `-o <file>`.

Options:

```text
-g   accepted and ignored in the Rust port (scripts ship as source, which keeps line numbers)
```

### Templates

The compiler implements the build-time half of `docs/template-syntax.md`:

- `<vpp-layout src="/layouts/main.html">` wraps the page in a layout. `<vpp-slot name="x">` in the layout receives `<vpp-fill slot="x">` from the page; the unnamed slot receives the page's other content. A slot's own children are its fallback. `mode="append"` or `mode="prepend"` on a fill adds to the fallback instead of replacing it. Layouts can use layouts.
- `<vpp-include src="/includes/footer.html">` pastes a fragment. Attributes become `{{ variables }}` inside it. `optional` makes a missing file expand to nothing.
- `<vpp-component src="/components/stat-card" value="3" label="pages">` expands `component.html` from that directory, with the attributes as `{{ properties }}` and the tag's children filling its slots. A boolean attribute (`highlight`) is the property `"true"`. Components can contain components and includes.
- A custom tag with a hyphen, such as `<stat-card>`, is a component when `vpp.json` lists it under `"components"` or a directory `components/<tag>/` exists.
- `component.css` is scoped: every element of the component's template gets a generated class, and every selector in the file requires it, so the component's rules cannot reach the rest of the page. `component.json` with `{"style": {"scoped": false}}` opts out.
- `{{ name }}` in text is replaced when the name is a known property or variable and left untouched otherwise. In attribute values an unknown name is removed, and an attribute that was only a placeholder is dropped.
- References beginning with `/` are from the project root; others are relative to the file that contains them. A reference that leaves the project directory is an error.
- Self-closing custom tags (`<stat-card ... />`) and `<vpp-slot>` inside `<head>` are handled by a text pre-pass before HTML parsing, since HTML itself has neither.

Not yet: expression properties, runtime bindings, events, conditionals, and component JavaScript. A `component.js` file produces a warning and is ignored. That is the runtime half of the specification and needs its own design first.

### Changes in the Rust port

- Ordinary element nesting no longer counts towards the 32-level include and component limit.
- An include at the top level of an included file is expanded.
- `data:` URLs and `#fragment` references are left as written.
- Scripts ship as JavaScript source in a `code.bin` container, not as bytecode (`docs/design/0001-code-bin.md`). `vppc` checks each for syntax errors and reports them as `file:line:col`. `-g` is accepted and ignored.

## The binary resources

- **dom.bin** holds the element tree with tags, attributes, and text. `<script>`, `<style>`, and `<link>` elements are removed, since their content lives in the other resources, and runs of whitespace collapse to one space.
- **style/NN-name.bin** holds one stylesheet already parsed: selectors with their specificity and the declarations. Selector matching still happens at run time, because scripts can change the DOM.
- **code/NN-name.bin** holds one script. The C++ compiler wrote QuickJS bytecode here; the Rust compiler writes the JavaScript source in a container (`docs/formats/code.md`).

Every decoder validates lengths and counts, so a truncated or corrupt file produces an error page rather than a crash.

## What compilation does and does not guarantee

- A distributed page's JavaScript is readable, as on the web. (The C++ tools shipped bytecode instead, which only raised the effort of reading it; it was never encryption.)
- Every script parses: syntax errors stop the build.
- Loading these resources is only safe from a trusted source. That is what package signing provides.

## Planned

- Assets: images and fonts packed with a virtual path table.
- ES modules, with imports followed across files.
- The runtime half of the template specification.
- A release build of the engine with the HTML, CSS, and JavaScript parsers compiled out.
- Private debug maps so stripped stack traces can be resolved on the developer's machine.
