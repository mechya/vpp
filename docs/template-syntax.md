# VPP — Viewer Package Platform

## HTML Template Syntax Specification

**Status:** Draft
**Target:** VPP 1.0
**Purpose:** Define reusable HTML includes, layouts, slots, and components for VPP applications.

See the appendix at the end for the implementation plan and the decisions still open.

---

# 1. Design Principles

VPP templates should follow these principles:

* Remain close to standard HTML.
* Be understandable without learning a full template language.
* Work without a server.
* Resolve static includes at build time where possible.
* Allow reusable components.
* Allow layouts with named slots.
* Support component properties.
* Support default slot content.
* Support named slots.
* Support nested components.
* Support scoped component styles.
* Support component JavaScript.
* Compile into VPP DOM/binary resources for release applications.
* Keep development files readable and debuggable.
* Produce useful compiler errors with source locations.

VPP-specific elements use the prefix:

```text
vpp-
```

This prevents conflicts with normal HTML elements.

---

# 2. Recommended Project Structure

```text
my-app/
│
├── vpp.json
│
├── pages/
│   ├── home.html
│   ├── profile.html
│   └── settings.html
│
├── layouts/
│   ├── main.html
│   ├── settings.html
│   └── fullscreen.html
│
├── components/
│   ├── app-header/
│   │   ├── component.html
│   │   ├── component.css
│   │   └── component.js
│   │
│   ├── app-footer/
│   │   ├── component.html
│   │   └── component.css
│   │
│   ├── user-card/
│   │   ├── component.html
│   │   ├── component.css
│   │   └── component.js
│   │
│   └── button/
│       ├── component.html
│       └── component.css
│
├── includes/
│   ├── copyright.html
│   ├── analytics-placeholder.html
│   └── navigation-links.html
│
├── styles/
│   └── global.css
│
├── scripts/
│   └── app.js
│
└── assets/
```

---

# 3. File Types

VPP recognizes four reusable UI concepts:

```text
Include
Component
Layout
Page
```

Their roles are different.

## Include

Simple HTML fragment insertion.

```html
<!--
Processing overview:
Inserts the copyright fragment into this location during compilation.
-->
<vpp-include src="/includes/copyright.html" />
```

Includes do not have independent state or lifecycle.

## Component

Reusable UI unit.

```html
<!--
Processing overview:
Renders a reusable user card with properties supplied by the parent page.
-->
<vpp-component
    src="/components/user-card"
    name="Bhupesh"
    role="Developer"
/>
```

Components may have:

* properties
* slots
* CSS
* JavaScript
* lifecycle events
* local DOM scope

## Layout

Defines common page/window structure.

```html
<!--
Processing overview:
Applies the common application layout to this page.
-->
<vpp-layout src="/layouts/main.html">
    ...
</vpp-layout>
```

Layouts normally contain one or more slots.

## Page

A final navigable or window-loadable document.

Examples:

```text
/pages/home.html
/pages/settings.html
/pages/about.html
```

---

# 4. Includes

## 4.1 Basic Include

Syntax:

```html
<!--
Processing overview:
Includes a static HTML fragment at build time.
-->
<vpp-include src="/includes/navigation-links.html" />
```

Compiler behavior:

```text
page.html
    +
navigation-links.html
    ↓
combined DOM
```

---

# 5. Include Path Rules

Absolute project path:

```html
<!--
Processing overview:
Loads a shared include from the project root.
-->
<vpp-include src="/includes/footer-links.html" />
```

Relative path:

```html
<!--
Processing overview:
Loads an include relative to the current source file.
-->
<vpp-include src="../includes/footer-links.html" />
```

VPP should normalize paths before compilation.

Directory traversal outside the project root must be rejected.

Invalid:

```text
../../../secret-file.html
```

---

# 6. Optional Includes

An include normally causes a build failure when its target is missing.

Optional syntax:

```html
<!--
Processing overview:
Loads the optional development notice when the file exists.
-->
<vpp-include
    src="/includes/dev-notice.html"
    optional
/>
```

If missing:

```text
optional include
→ ignored
```

If `optional` is not present:

```text
missing include
→ compiler error
```

---

# 7. Includes With Variables

Simple includes may receive parameters.

```html
<!--
Processing overview:
Includes a reusable page heading and provides its text value.
-->
<vpp-include
    src="/includes/page-heading.html"
    title="Settings"
/>
```

`page-heading.html`:

```html
<!--
Processing overview:
Displays the heading supplied by the including page.
-->
<h1>{{ title }}</h1>
```

This provides a lightweight alternative to a full component when no component lifecycle is needed.

---

# 8. Layouts

A layout defines reusable page structure.

Example:

`layouts/main.html`

```html
<!DOCTYPE html>
<html>
<head>
    <!--
    Processing overview:
    Provides common metadata and allows pages to add page-specific head content.
    -->
    <meta charset="utf-8">

    <vpp-slot name="head" />

    <link rel="stylesheet" href="/styles/global.css">
</head>

<body>

    <vpp-component src="/components/app-header" />

    <div class="application-layout">

        <aside>
            <vpp-slot name="sidebar">
                <!--
                Processing overview:
                Supplies fallback sidebar content when a page does not provide one.
                -->
                <p>No sidebar content</p>
            </vpp-slot>
        </aside>

        <main>
            <vpp-slot />
        </main>

    </div>

    <vpp-component src="/components/app-footer" />

    <vpp-slot name="scripts" />

</body>
</html>
```

---

# 9. Applying a Layout

`pages/home.html`:

```html
<!--
Processing overview:
Builds the home page using the application's main layout.
-->
<vpp-layout src="/layouts/main.html">

    <vpp-fill slot="head">
        <title>Home</title>
    </vpp-fill>

    <vpp-fill slot="sidebar">

        <nav>
            <a href="/home">Home</a>
            <a href="/profile">Profile</a>
        </nav>

    </vpp-fill>

    <vpp-fill>

        <h1>Welcome</h1>

        <p>
            Welcome to VPP.
        </p>

    </vpp-fill>

</vpp-layout>
```

---

# 10. Default Slot

A layout may contain:

```html
<!--
Processing overview:
Receives the page's primary unnamed content.
-->
<vpp-slot />
```

A page provides it using:

```html
<!--
Processing overview:
Provides content for the layout's default slot.
-->
<vpp-fill>

    <h1>Dashboard</h1>

</vpp-fill>
```

There may be only one default slot per layout or component.

---

# 11. Named Slots

Layout:

```html
<!--
Processing overview:
Defines separate regions that pages can populate independently.
-->
<header>
    <vpp-slot name="header" />
</header>

<aside>
    <vpp-slot name="sidebar" />
</aside>

<main>
    <vpp-slot />
</main>

<footer>
    <vpp-slot name="footer" />
</footer>
```

Page:

```html
<!--
Processing overview:
Provides content for several named layout regions.
-->
<vpp-layout src="/layouts/main.html">

    <vpp-fill slot="header">
        <h1>My Application</h1>
    </vpp-fill>

    <vpp-fill slot="sidebar">
        <nav>...</nav>
    </vpp-fill>

    <vpp-fill>
        <p>Main content</p>
    </vpp-fill>

    <vpp-fill slot="footer">
        <small>Copyright 2026</small>
    </vpp-fill>

</vpp-layout>
```

---

# 12. Slot Default Content

A slot can provide fallback content.

```html
<!--
Processing overview:
Displays application navigation unless the consuming page overrides it.
-->
<vpp-slot name="sidebar">

    <nav>
        <a href="/">Home</a>
    </nav>

</vpp-slot>
```

If the page provides:

```html
<vpp-fill slot="sidebar">
    ...
</vpp-fill>
```

the fallback content is replaced.

If it does not, fallback content remains.

---

# 13. Optional Slots

A slot without fallback content naturally renders nothing:

```html
<!--
Processing overview:
Provides an optional region for page-specific actions.
-->
<vpp-slot name="actions" />
```

No special `optional` attribute is necessary.

---

# 14. Append Slots

Sometimes a page should add content instead of replacing the layout content.

Layout:

```html
<!--
Processing overview:
Provides common scripts while permitting pages to append additional scripts.
-->
<vpp-slot name="scripts">

    <script src="/scripts/common.js"></script>

</vpp-slot>
```

Page:

```html
<!--
Processing overview:
Adds a page-specific script after the layout's common scripts.
-->
<vpp-fill
    slot="scripts"
    mode="append"
>

    <script src="/scripts/settings.js"></script>

</vpp-fill>
```

Supported modes:

```text
replace
append
prepend
```

Default:

```text
replace
```

---

# 15. Reusable Components

VPP components are directories.

Example:

```text
components/
└── user-card/
    ├── component.html
    ├── component.css
    └── component.js
```

---

# 16. Basic Component

`component.html`:

```html
<!--
Processing overview:
Defines a reusable card that displays information about one user.
-->
<article class="user-card">

    <h2>{{ name }}</h2>

    <p>{{ role }}</p>

</article>
```

Usage:

```html
<!--
Processing overview:
Creates a user card and passes user information as component properties.
-->
<vpp-component
    src="/components/user-card"
    name="Bhupesh"
    role="Developer"
/>
```

Rendered result:

```html
<article class="user-card">

    <h2>Bhupesh</h2>

    <p>Developer</p>

</article>
```

---

# 17. Component Registration Shortcut

VPP may allow component aliases in `vpp.json`.

```json
{
    "components": {
        "user-card": "/components/user-card",
        "app-header": "/components/app-header",
        "app-footer": "/components/app-footer"
    }
}
```

Then this:

```html
<!--
Processing overview:
Uses the registered user-card component alias.
-->
<user-card
    name="Bhupesh"
    role="Developer"
/>
```

is equivalent to:

```html
<vpp-component
    src="/components/user-card"
    name="Bhupesh"
    role="Developer"
/>
```

Custom VPP component names must contain a hyphen.

Recommended naming:

```text
user-card
app-header
settings-panel
file-browser
project-tree
```

---

# 18. Component Property Declaration

Components should explicitly declare accepted properties.

`component.html`:

```html
<!--
Processing overview:
Declares the public properties accepted by the user-card component.
-->
<vpp-props>

    <vpp-prop
        name="name"
        type="string"
        required
    />

    <vpp-prop
        name="role"
        type="string"
        default="User"
    />

    <vpp-prop
        name="active"
        type="boolean"
        default="false"
    />

</vpp-props>


<article class="user-card">

    <h2>{{ name }}</h2>

    <p>{{ role }}</p>

</article>
```

---

# 19. Supported Property Types

VPP 1.0 should support:

```text
string
number
integer
boolean
object
array
json
```

Examples:

```html
<vpp-prop
    name="count"
    type="integer"
    default="0"
/>

<vpp-prop
    name="enabled"
    type="boolean"
    default="true"
/>

<vpp-prop
    name="items"
    type="array"
/>
```

---

# 20. Literal Properties

```html
<!--
Processing overview:
Passes literal string, number, and boolean properties to the component.
-->
<user-card
    name="Bhupesh"
    age="30"
    active
/>
```

Boolean shorthand:

```html
active
```

means:

```text
active = true
```

---

# 21. Expression Properties

Use `:` before the property name for expressions.

```html
<!--
Processing overview:
Passes runtime JavaScript values instead of literal strings.
-->
<user-card
    :name="currentUser.name"
    :role="currentUser.role"
    :active="currentUser.active"
/>
```

Meaning:

```text
name   ← currentUser.name
role   ← currentUser.role
active ← currentUser.active
```

This keeps literal values and executable expressions clearly separated.

---

# 22. Text Expressions

VPP uses:

```text
{{ expression }}
```

Example:

```html
<!--
Processing overview:
Displays application data inside normal HTML content.
-->
<h1>Hello {{ user.name }}</h1>
```

Expressions should support normal JavaScript expressions supported by the VPP runtime.

---

# 23. Attribute Expressions

Use the `:` prefix.

```html
<!--
Processing overview:
Binds element attributes to runtime application values.
-->
<img
    :src="user.avatar"
    :alt="user.name"
/>
```

---

# 24. Boolean Attributes

```html
<!--
Processing overview:
Disables the button when the application is currently saving.
-->
<button :disabled="saving">
    Save
</button>
```

If `saving` is false, the attribute is removed.

---

# 25. Component Default Slot

Component:

```html
<!--
Processing overview:
Defines a panel component whose body comes from the calling page.
-->
<section class="panel">

    <header>
        <h2>{{ title }}</h2>
    </header>

    <div class="panel-body">
        <vpp-slot />
    </div>

</section>
```

Usage:

```html
<!--
Processing overview:
Places custom body content inside the panel's default slot.
-->
<app-panel title="Account">

    <p>Account settings appear here.</p>

</app-panel>
```

---

# 26. Component Named Slots

Component:

```html
<!--
Processing overview:
Defines a card with separate header, content, and footer regions.
-->
<article class="card">

    <header>
        <vpp-slot name="header" />
    </header>

    <section>
        <vpp-slot />
    </section>

    <footer>
        <vpp-slot name="footer" />
    </footer>

</article>
```

Usage:

```html
<!--
Processing overview:
Supplies custom content for each user-card slot.
-->
<user-card>

    <vpp-fill slot="header">
        <h2>Bhupesh</h2>
    </vpp-fill>

    <vpp-fill>
        <p>Web System Engineer</p>
    </vpp-fill>

    <vpp-fill slot="footer">
        <button>View Profile</button>
    </vpp-fill>

</user-card>
```

---

# 27. Slot Shorthand

For components only, VPP may support:

```html
<!--
Processing overview:
Uses slot shorthand syntax for a named component region.
-->
<user-card>

    <div vpp-slot="header">
        <h2>Bhupesh</h2>
    </div>

    <p>Developer</p>

</user-card>
```

However, canonical VPP syntax should remain:

```html
<vpp-fill slot="header">
```

because it is clearer for compilation and documentation.

---

# 28. Nested Components

Components can contain other components.

```html
<!--
Processing overview:
Builds the application header from smaller reusable VPP components.
-->
<header class="app-header">

    <app-logo />

    <navigation-menu />

    <user-avatar
        :user="currentUser"
    />

</header>
```

The compiler resolves the dependency tree automatically.

---

# 29. Component CSS

`components/user-card/component.css`:

```css
/*
 * Processing overview:
 * Defines visual styling belonging only to the user-card component.
 */

.user-card {
    padding: 16px;
    border-radius: 12px;
}

.user-card h2 {
    margin: 0;
}
```

By default, VPP should scope component CSS.

Conceptually:

```css
.user-card[data-vpp-scope="A73F"] {
    padding: 16px;
}
```

This prevents one component stylesheet from unexpectedly changing another component.

---

# 30. Global Component CSS

A component may explicitly opt out of CSS scoping.

`component.json`:

```json
{
    "style": {
        "scoped": false
    }
}
```

Global CSS should be discouraged for reusable components.

---

# 31. Component JavaScript

`component.js`:

```javascript
/*
 * Processing overview:
 * Controls user-card interactions and exposes the component's local behavior.
 */

export function setup(component) {

    const button = component.querySelector(".profile-button");

    if (!button) {
        return;
    }

    button.addEventListener("click", () => {
        component.emit("profile-open");
    });
}
```

Component JavaScript should run in the component's local context.

---

# 32. Component Events

A component can emit events:

```javascript
/*
 * Processing overview:
 * Emits a save event so the parent page can react to the component action.
 */

component.emit("save", {
    id: user.id
});
```

Parent:

```html
<!--
Processing overview:
Listens for the component save event and calls the page handler.
-->
<user-editor
    @save="handleUserSave"
/>
```

Event syntax:

```text
@event="handler"
```

Examples:

```html
@save="handleSave"

@close="closeDialog"

@change="updateValue"
```

---

# 33. Native DOM Events

The same syntax may be used for DOM events:

```html
<!--
Processing overview:
Calls the application save handler when the button is clicked.
-->
<button @click="saveDocument">
    Save
</button>
```

Equivalent JavaScript behavior:

```javascript
button.addEventListener("click", saveDocument);
```

---

# 34. Event With Expression

```html
<!--
Processing overview:
Removes the selected record when the button is clicked.
-->
<button @click="removeItem(item.id)">
    Remove
</button>
```

---

# 35. Conditional Rendering

Although not strictly required for includes/layouts, components need basic conditional rendering.

Canonical syntax:

```html
<!--
Processing overview:
Shows account information only when a user is signed in.
-->
<vpp-if test="user">

    <user-card :user="user" />

</vpp-if>
```

With fallback:

```html
<!--
Processing overview:
Displays either authenticated user content or the login prompt.
-->
<vpp-if test="user">

    <user-card :user="user" />

    <vpp-else>
        <login-panel />
    </vpp-else>

</vpp-if>
```

---

# 36. Further sections

Sections from 36 onward (list rendering, element references, lifecycle, and the runtime API) are not yet written.

---

# Appendix: Implementation Plan and Open Decisions

This appendix records how the specification maps onto the VPP compiler and runtime as they exist today, and the decisions to settle before implementation starts.

## Two parts of very different size

**Part A, build-time templating.** Sections 4 to 17, 20, 25 to 27, 29 and 30: includes with `optional` and variables, layouts with slots and fills and the replace/append/prepend modes, components with literal properties, default and named slots, nested components, scoped component CSS, aliases from `vpp.json`, path rules, and errors with source locations. Every one of these resolves to a plain DOM at compile time. The viewer never knows templates existed. This fits the current compiler directly: template expansion becomes one pass between parsing the page and encoding `dom.bin`, and component CSS is appended to `style.bin`.

**Part B, runtime templating.** Sections 18 and 19 as far as types are checked at run time, 21 to 24, 28 where expression props are used, and 31 to 35: expression properties, text and attribute expressions over runtime data, boolean attribute bindings, event bindings, conditional rendering, component JavaScript with `setup`, `emit`, and `querySelector`, and lifecycle. None of this resolves at build time. It needs the compiler to emit JavaScript that builds and updates DOM, a data model, an update strategy, a much larger DOM API in the runtime, custom events, and ES module support. This is a framework in its own right and gets its own design document before implementation. The first two decisions in that document are the brace rule below and the update model: explicit `component.update()` calls are the simplest choice for 1.0, with fine-grained reactivity deferred.

Part A is planned first.

## Decisions to settle before writing any component

1. **What `{{ }}` means.** Section 7 uses it for build-time include variables, section 16 for component properties, and section 22 for runtime expressions. The compiler cannot tell these apart from the syntax. Proposed rule: a property passed as a literal string is substituted at build time; any expression the compiler cannot resolve to a literal is a runtime expression and requires Part B. Alternatively, use different delimiters for the two. Changing this later breaks every application, so it is decided first.

2. **Self-closing custom tags do not exist in HTML.** An HTML5 parser reads `<vpp-include src="x" />` as an opening tag, ignores the slash, and swallows everything after it as children. The compiler will run a pre-pass that expands self-closing tags whose names contain a hyphen into open and close pairs before parsing. Authors keep writing `/>`.

3. **Slots inside `<head>` are moved by the parser.** An HTML parser meeting an unknown element inside `<head>` closes the head and starts the body, so the head slot in the section 8 layout example would silently move. The pre-pass will handle `vpp-` elements inside `<head>`, or the layout syntax will declare head content through a dedicated element the compiler recognises. To be decided.

4. **Scoped CSS as written needs attribute selectors**, which the engine does not support yet. Scoping will add a generated class to the component's root and to each selector in its stylesheet, which gives the same isolation with the selectors the engine already matches. The attribute form can come when attribute selectors do.

## Smaller notes

- Component JavaScript with `export function setup` needs ES modules. The compiler currently concatenates scripts into one program; module compilation is a Part B prerequisite.
- Component aliases in `vpp.json` need the manifest reader to accept a nested `components` object. Small extension.
- Custom element names must contain a hyphen, as the spec says; that is also how the compiler will distinguish components from HTML elements without a registry lookup.
- The `hello-world` example will be restructured into pages, layouts, and components as the Part A test case.
- **One `.vpp` per page.** Each file under `pages/` compiles to its own package with the layout and components expanded into it. A site is a folder of page packages that link to each other with `<a href="profile.vpp">`; the viewer downloads pages on demand, keeps them until they change, and shares resources between pages by hash. See the root README, "Pages, Sites, and Downloads".
