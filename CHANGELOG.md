# Changelog

## 0.2.0

A clean break from 0.1. The derive, its generated items and the runtime
types are all new; nothing from 0.1 is kept or deprecated.

### Added

- `#[derive(Fields)]` with the field attributes `#[fields(nested)]`,
  `#[fields(skip)]`, `#[fields(indexed)]` and `#[fields(indexed, nested)]`.
- Generated per struct `X`: the path enum `XPath` and the write handle
  `XMut<'a>`, both with the visibility of `X`, and `impl Fields for X`.
- Runtime items: `Fields`, `Path`, `Words`, `Changes`, `Drain`, `Iter`,
  `Field`, `Index`, `Mask` and `MaskOps`.
- `Changes<X>`: a bitset of one bit per leaf with `const fn new`, `track`,
  `drain`, `iter`, `mark_all`, `clear`, `len` and `is_empty`.
- `Field`: `get`, `set`, `set_if_changed` and `update` on one leaf.
- Compile errors, each on the offending struct or field, for every rejected
  input shape and reserved-name collision, pinned by a compile-fail suite.
- Builds on stable Rust; the minimum supported version is 1.88.
- `no_std` without `alloc`, and `#![forbid(unsafe_code)]`.

### Removed

- The `FieldSet` derive and the `#[fieldset]` and `#[fieldset_skip]`
  attributes.
- The generated `*FieldType` enums and `*FieldSetter` traits.
- `apply`, `IntoIterator for X` and the `*_VARIANCE` constants.
- `OptFieldSet`, `BitFieldSet` and `PerfFieldSet`.
- `BitSet` and `BitSetOffsetted`.
- The `convert_case` and `version-sync` dependencies.
- The nightly requirement (`impl_trait_in_assoc_type`).

### Changed

- Events carry no values. A drained path names the leaf; the reader takes
  the value from the model.
- Package metadata lives in `[workspace.package]`; `fieldset` depends on
  `fieldset_macro` with an exact version.
