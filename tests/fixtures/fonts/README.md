# Test fonts

Fonts used by tests only, so text measures and renders the same on every machine. The viewer uses the system's fonts (`docs/reference/viewer.md`); these are never shipped with it.

| File | Font | Version | SHA-256 |
|---|---|---|---|
| `DejaVuSans.ttf` | DejaVu Sans | 2.37 | `7da195a74c55bef988d0d48f9508bd5d849425c1770dba5d7bfc6ce9ed848954` |
| `DejaVuSans-Bold.ttf` | DejaVu Sans Bold | 2.37 | `e6476c1b80502924294eed40894c5b18e06c181444ca953e5334262df9c27724` |

Downloaded from the official release, https://github.com/dejavu-fonts/dejavu-fonts/releases/tag/version_2_37 (`dejavu-fonts-ttf-2.37.zip`), unmodified.

**Licence:** Bitstream Vera Fonts licence, with DejaVu's changes in the public domain; see `LICENSE-DejaVu.txt`. It allows copying and redistributing the fonts, with the notice kept; they may not be sold on their own.

Like every fixture, these files are frozen: screenshot tests depend on their exact glyphs. Changing them changes the reference images.
