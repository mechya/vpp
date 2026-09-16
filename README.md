# VPP — Viewer Package Platform

**VPP (Viewer Package Platform)** is an experimental lightweight web viewer and application platform.

It combines familiar web technologies with a lightweight native viewer, application runtime, packaging system, secure update mechanism, and development tools.

## Core Idea

Development:

```text
HTML
CSS
JavaScript
   ↓
VPP Compiler
   ↓
Compiled binary resources
   ↓
.vpp package
```

Execution:

```text
.vpp
 ↓
VPP Viewer
 ↓
Verify package
 ↓
Load resources
 ↓
Execute compiled JavaScript
 ↓
Display application
```

A normal web address can also be opened directly inside VPP Viewer.

## Viewer Concept

VPP Viewer initially behaves as a minimal browser.

When opened without content:

```text
┌───────────────────────────────────────────────┐
│ Search or enter address                      │
├───────────────────────────────────────────────┤
│                                               │
│                                               │
│                                               │
└───────────────────────────────────────────────┘
```

The address bar can be used for:

- URLs
- Web searches
- VPP links
- Local `.vpp` applications

After a page or VPP application is opened, the address bar can automatically hide.

`Ctrl + L` can reveal and focus it again.

## Design Goals

VPP aims to provide:

- Minimal frameless viewer
- Normal web browsing
- HTML and CSS based interfaces
- Existing JavaScript development workflow
- JavaScript compilation into binary form
- `.vpp` application packaging
- Offline application execution
- Signed applications
- Encrypted application resources
- Incremental updates
- Multi-window support
- Multi-instance support
- Optional tab support
- Developer debugging tools
- Cross-platform runtime architecture

## Repository Structure

```text
vpp/
│
├── README.md
├── LICENSE
├── .gitignore
│
├── viewer/
│   └── README.md
│
├── runtime/
│   └── README.md
│
├── compiler/
│   └── README.md
│
├── packager/
│   └── README.md
│
├── updater/
│   └── README.md
│
├── sdk/
│   └── README.md
│
├── tools/
│   └── README.md
│
├── examples/
│   └── README.md
│
├── tests/
│
└── docs/
```

## Components

### VPP Viewer

`viewer/`

Native VPP application responsible for displaying:

- normal websites
- VPP applications
- address/search interface
- windows
- tabs
- navigation

### VPP Runtime

`runtime/`

Execution environment used by VPP applications.

Responsibilities include:

- JavaScript binary execution
- DOM interaction
- application lifecycle
- event handling
- runtime permissions
- storage APIs
- window APIs
- networking APIs

### VPP Compiler

`compiler/`

Converts application source code into VPP-compatible compiled resources.

Initial target:

```text
JavaScript
   ↓
VPP Compiler
   ↓
code.bin
```

Development source files remain readable during development but do not need to be distributed with release applications.

### VPP Packager

`packager/`

Creates the final `.vpp` application package.

Example:

```text
Application source
       ↓
     build
       ↓
Compiled resources
       ↓
    package
       ↓
weather.vpp
```

### VPP Updater

`updater/`

Handles signed incremental application updates.

Only files that changed should need to be downloaded.

Example:

```text
Installed

ui.bin       unchanged
style.bin    unchanged
code.bin     changed
assets.bin   unchanged

Download

code.bin only
```

### VPP SDK

`sdk/`

Defines APIs available to VPP applications.

Possible APIs include:

```text
VPP.window
VPP.storage
VPP.files
VPP.network
VPP.permissions
VPP.system
VPP.update
```

### VPP Tools

`tools/`

Development utilities such as:

- debugger
- inspector
- package inspector
- signature verifier
- compiler tools
- development launcher

### Examples

`examples/`

Contains small applications used to test VPP features and demonstrate development patterns.

## VPP Package

A future `.vpp` application could internally contain resources such as:

```text
weather.vpp
│
├── manifest.bin
├── dom.bin
├── style.bin
├── code.bin
└── assets.bin
```

The exact package specification will be defined as the project develops.

## Security Model

Release VPP applications may use:

- cryptographic hashes
- publisher signatures
- encrypted binary resources
- application identifiers
- version verification
- rollback protection

A VPP Viewer should execute an application update only after verifying its publisher signature.

## Incremental Updates

VPP applications should not require downloading the entire package after every update.

The update system should compare resource hashes.

```text
local hash
    ↓
compare
    ↓
remote hash
```

Only changed resources are downloaded.

## Development Mode

During development, applications remain easy to inspect and debug.

```text
HTML + CSS + JavaScript
          ↓
      VPP Dev Mode
          ↓
┌─────────────────────────────┐
│ Elements                    │
│ Console                     │
│ Sources                     │
│ Network                     │
│ Storage                     │
│ VPP Runtime                 │
└─────────────────────────────┘
```

Development mode may support:

- source debugging
- breakpoints
- console logging
- HTML inspection
- CSS inspection
- live reload
- hot reload
- runtime diagnostics

## Release Mode

A release build may perform:

```text
Validate
   ↓
Compile
   ↓
Optimize
   ↓
Generate binary resources
   ↓
Encrypt
   ↓
Hash
   ↓
Sign
   ↓
Package
   ↓
application.vpp
```

Debug information should normally remain outside the distributed package.

## Initial Development Roadmap

### Phase 1 — Viewer

- Native application window
- Frameless window
- Address/search bar
- Web rendering
- URL navigation
- Search engine integration
- Address bar auto-hide
- `Ctrl + L`
- Basic navigation

### Phase 2 — Runtime

- Application lifecycle
- Runtime API
- DOM bridge
- JavaScript execution

### Phase 3 — Compiler

- JavaScript compiler integration
- Binary JavaScript representation
- Debug mapping

### Phase 4 — Package

- `.vpp` specification
- Manifest
- Resource packaging
- Local application loading

### Phase 5 — Security

- Publisher keys
- Digital signatures
- Resource hashes
- Encryption
- Rollback protection

### Phase 6 — Updates

- Update manifest
- Changed-resource detection
- Incremental downloads
- Atomic resource replacement

## Status

VPP is currently in the architecture and early prototyping stage.

The first implementation target is the VPP Viewer.
