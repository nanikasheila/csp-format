# Changelog

All notable changes to the crates `csp-format`, `csp-assets`, `cspf` and the
Python package `csp-format`. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (before 1.0, breaking changes
raise the minor version).

## [Unreleased]

## [0.2.0] - 2026-11-10

### Added

- `csp-format`: what a writer of pages needed SQL of its own for.
  - 見開き: `ClipFile::make_double_page` (`story::make_double_page`,
    `story::set_thumbnail_canvas_size`, `story::SPREAD_FRAME_FLAG`,
    `story::SPREAD_CANVAS_FLAGS`) makes a single page's canvas that of a
    spread. The format is the new chapter `docs/spec/10-spread.md`.
  - The stored preview: `ClipFile::preview_png` / `set_preview`
    (`edit::preview_png`, `edit::set_preview`) read it and replace it by an
    image made some other way than `update_preview`, with its size.
  - Text: `ClipFile::layer_text` / `set_layer_text` (`story::layer_text`,
    `story::set_layer_text`) replace the text of a one-run text layer with its
    run length and without the image CSP cached for it.
  - Offscreens: `ClipFile::set_offscreen` replaces an offscreen's `Attribute`
    and, with pixels, its external chunk (a layer mask's levels, a canvas
    made wider). `model::LAYER_MASK_ENABLED` names the `LayerVisibility` bit,
    and `docs/spec/03-canvas-and-layers.md` says how a mask is written.
  - Rows by name: `database::row`, `first_row`, `update_row`, `update_rows`
    and `add_column`, for the columns that have no function of their own.
- `csp-assets`: write a sub tool from its settings. `SubToolDb::sut_parts`
  gives one sub tool's manager, node, settings rows and packed materials, and
  `build_sut` writes such parts as a `.sut` for CSP's サブツールの読み込み, each
  `Variant` column declared with the type CSP uses (`variant_column_type`,
  generated from `docs/assets/spec/subtool-columns.tsv` by
  `tools/assets/subtool_column_types.py`), so a value is stored as CSP stores it.

### Changed

- rusqlite 0.41 (its bundled SQLite fixes RUSTSEC-2026-0459 /
  CVE-2026-119375 in `libsqlite3-sys` 0.38). The minimum Rust version is now
  1.89, as rusqlite 0.41 needs it. `database::serialize` keeps taking
  `&Connection`: it asks SQLite for a copied image itself, since
  `Connection::serialize` now needs `&mut`.
- `docs/assets/spec/subtool-columns.tsv`: the value 2 of the length unit
  columns (`BrushSizeUnit`, `BrushInLengthUnit`...) is tens of pixels, not
  millimetres: CSP shows `BrushSize` 1.2 with unit 2 as 12.0 px and draws it
  as wide as 12 px on 600 and 1200 dpi canvases alike (W3c, checked for
  `BrushSizeUnit`).
- Development and CI use the Rust version in `rust-toolchain.toml`. CI also
  looks for known vulnerabilities (`cargo audit`), runs with read-only
  permissions (only the release job may write), and uses Actions pinned by
  commit (`tools/release/workflow_pins.py` checks it). Dependabot proposes
  the updates of crates, Actions and the Rust version.
- The repository moved from `nanikasheila/CLIP_STUDIO_PAINT_Format` to
  `nanikasheila/csp-format`, so that its name matches the packages and does
  not carry the product name. GitHub forwards the old URLs. The repository
  and homepage links in the crate and PyPI metadata change with the next
  release; the 0.1.0 pages keep the old links, which still work.

## [0.1.0] - 2026-10-26

First release.

### csp-format

- Read `.clip`: the CSFCHUNK container, the embedded SQLite database and its
  schema registry, the canvas and layer tree, raster pixels (colour, gray,
  1-bit, masks, mipmaps), vector data, text layers, layer effects, tone,
  gradient, correction and image material layers, the stored preview, and
  animation (timelines, cels, keyframes, the 2D camera, onion skin).
- Read `.cmc`: the page list in reading order and the work settings.
- Write: lossless saving (an unchanged file is byte-identical), new canvases
  and works with story info and nombre, editing pixels and layer attributes,
  adding and removing layers and folders, copying layers between files,
  page list edits, animation timeline and cel edits.
- Render the layers into an image; check every part of a file
  (`verify::check_clip`, `verify::check_cmc`); tell which CSP generation
  saved a file (`version`).

### csp-assets

- Sub tools (`.sut`) and tool databases (`.todb`), auto actions (`.las`,
  `.laf`), colour sets (`.cls`), gradient sets (`.sgs`), shortcut and
  modifier key settings, material folders, the C2F envelope of encrypted
  material data (never decrypted).

### cspf

- Commands to inspect, check, extract, render, create and edit files, and
  `cspf asset` for the files of `csp-assets`.
- Inputs are never replaced: a command refuses an output that is one of its
  inputs, by any path, symbolic link or hard link.

### Python

- `csp_format`: a reader with the standard library only, matching the Rust
  crate; `csp_format.assets` for the files of `csp-assets`.
- The sdist carries the tests and their synthetic files.

[Unreleased]: https://github.com/nanikasheila/csp-format/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nanikasheila/csp-format/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nanikasheila/csp-format/releases/tag/v0.1.0
