# Security Policy

VPP verifies publisher signatures, pins publisher keys, refuses rollbacks, and parses untrusted packages, HTML, CSS, and SVG. A flaw in any of these can let a hostile page or server harm the people using the viewer, so please report it privately.

## Reporting a vulnerability

**Do not open a public issue.** Report it through GitHub's private vulnerability reporting:

**https://github.com/mechya/vpp/security/advisories/new**

Include what you can of:

* what an attacker can do, and what they need (a hostile `.vpp`, a hostile server, a network position)
* the viewer or tool version, and the operating system
* steps or a file that reproduce it

You will get an answer within 7 days. Once a fix is ready, it is released first and the advisory is published afterwards, crediting you unless you prefer not to be named.

## In scope

* Signature, publisher-key, and hash checks that can be bypassed
* Rollback protection that can be bypassed
* Crashes, hangs, or memory corruption from a package, HTML, CSS, SVG, font, or network response
* A page escaping its limits: faking the shell or address bar, reaching another site's storage, or calling APIs it was not granted
* Weaknesses in the updater's downloads

## Supported versions

VPP is before 1.0. Only the latest release and `main` receive security fixes.
