# Patched dependencies

Local copies of crates.io dependencies that carry changes of our own. Each one is wired up in the
root `Cargo.toml` under `[patch.crates-io]`. Each patch is described below; add new ones as a new
section.

## parley

- **Upstream:** `parley` 0.8.0 from crates.io (monorepo `linebender/parley`, commit `8dbecc0`,
  path `parley`)
- **Where:** `patches/parley/` is the unpacked crate with one modified file. Everything else is
  identical to upstream.
- **Why:** Upstream builds its segmenters with `new_for_non_complex_scripts`, which leaves out the
  dictionary data for scripts that have no spaces between words (e.g. Thai, Lao, Khmer, Burmese,
  and CJK word breaking). Text in those scripts is then not broken at word boundaries when it is
  wrapped or when the cursor jumps by word. We want that to work, so the segmenters use
  `new_dictionary`.
- **What changed:** only `src/analysis/mod.rs`:
  - `word_segmenter()`: `WordSegmenter::new_for_non_complex_scripts(..)` becomes
    `WordSegmenter::new_dictionary(..)`.
  - `line_segmenter()`: `LineSegmenter::new_for_non_complex_scripts(..)` becomes
    `LineSegmenter::new_dictionary(..)` for `Normal`, `BreakAll` and `KeepAll`.
  - The `const { .. }` blocks are removed, since `new_dictionary` is not a `const fn`.
- **Cost:** larger binary (dictionary data), and the segmenters are built at runtime instead of
  at compile time.
- **Updating parley:** download the new version, copy it over `patches/parley/`, then repeat the
  change above in `src/analysis/mod.rs`. Check whether upstream has an option for this by then,
  and if so drop the patch. To see the exact diff against upstream:
  `curl -sL https://static.crates.io/crates/parley/parley-0.8.0.crate | tar xz` and
  `diff -ru parley-0.8.0 patches/parley`.
