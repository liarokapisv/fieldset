# fieldset

[![CI](https://github.com/liarokapisv/fieldset/actions/workflows/ci.yml/badge.svg)](https://github.com/liarokapisv/fieldset/actions)
[![docs.rs](https://docs.rs/fieldset/badge.svg)](https://docs.rs/fieldset)
[![crates.io](https://img.shields.io/crates/v/fieldset.svg)](https://crates.io/crates/fieldset)

Typed field paths and change tracking for plain structs.

The model stays a plain struct. `#[derive(Fields)]` adds nothing to it and
generates three separate items: a path enum that names every leaf, a write
handle whose field-named methods write a leaf and record that it was written,
and the glue that lets `Changes<Model>` hold one bit per leaf. Draining the
set yields the paths of the leaves that were written, in declaration order,
so a reactor can read the current value straight from the model.

The crate is `no_std`, allocates nothing, contains no `unsafe`, and builds on
stable Rust 1.88 or newer.

## Example

```rust
use fieldset::{Changes, Fields};

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    pub cutoff: f32,
    pub resonance: f32,
}

#[derive(Fields, Debug)]
pub struct Preset {
    pub volume: f32,
    #[fields(nested)]
    pub filter: Filter,
    #[fields(skip)]
    pub name: [u8; 16],
}

let mut preset = Preset {
    volume: 0.5,
    filter: Filter { cutoff: 0.3, resonance: 0.1 },
    name: [0; 16],
};
let mut changes = Changes::<Preset>::new();

{
    let mut p = changes.track(&mut preset);
    p.volume().set(0.8);
    p.filter().cutoff().update(|cutoff| *cutoff += 0.1);
    p.filter().resonance().set_if_changed(0.1);
}

let written: Vec<PresetPath> = changes.drain().collect();
assert_eq!(
    written,
    [PresetPath::Volume, PresetPath::Filter(FilterPath::Cutoff)]
);
assert!(changes.is_empty());

for path in written {
    match path {
        PresetPath::Volume => println!("volume is now {}", preset.volume),
        PresetPath::Filter(FilterPath::Cutoff) => println!("cutoff is now {}", preset.filter.cutoff),
        PresetPath::Filter(FilterPath::Resonance) => unreachable!("not written"),
    }
}
```

Writing `resonance` with the value it already held marked nothing, so the
drain yielded two paths. `Preset` itself is untouched by the derive: it has
the three fields it declares and nothing else.

## What the derive generates

For a struct `X` with the visibility of `X`:

| Item | Role |
|---|---|
| `XPath` | An enum with one unit variant per leaf and one tuple variant per nested or indexed field. `Copy`, `Eq`, `Debug`. Takes the visibility of `X`. |
| `XMut<'a>` | The write handle: one method per field, named like the field, plus `model()`, `set(X)` and `mark_all()`. Takes the visibility of `X`. |
| `impl Fields for X` | `COUNT`, the word storage, and the two types above. |

Leaves are numbered in declaration order, depth first: a nested field
occupies the contiguous range of its own leaves.

## Field attributes

| Attribute | Field type | Leaves | Handle method | Path variant |
|---|---|---|---|---|
| none | `T: Copy` | 1 | `f(&mut self) -> Field<'_, T>` | `F` |
| `#[fields(nested)]` | `S: Fields` | `S::COUNT` | `f(&mut self) -> SMut<'_>` | `F(SPath)` |
| `#[fields(skip)]` | anything | 0 | none | none |
| `#[fields(indexed)]` | `[T; N]`, `T: Copy` | `N` | `f(&mut self, Index<N>) -> Field<'_, T>` | `F(Index<N>)` |
| `#[fields(indexed, nested)]` | `[S; N]`, `S: Fields` | `N * S::COUNT` | `f(&mut self, Index<N>) -> SMut<'_>` | `F(Index<N>, SPath)` |

A leaf is any `Copy` value, including enums with payloads and arrays that are
not marked `indexed`; the derive never looks inside a leaf.

```rust
use fieldset::{Changes, Fields, Index};

#[derive(Fields, Clone, Copy)]
pub struct Voice {
    pub pitch: f32,
    pub gate: bool,
}

#[derive(Fields)]
pub struct Synth {
    #[fields(indexed, nested)]
    pub voices: [Voice; 6],
    #[fields(indexed)]
    pub sends: [f32; 2],
}

let mut synth = Synth {
    voices: [Voice { pitch: 0.0, gate: false }; 6],
    sends: [0.0; 2],
};
let mut changes = Changes::<Synth>::new();

let third = Index::<6>::new(2).expect("in range");
let second = Index::<2>::new(1).expect("in range");
changes.track(&mut synth).voices(third).gate().set(true);
changes.track(&mut synth).sends(second).set(0.5);

let written: Vec<SynthPath> = changes.drain().collect();
assert_eq!(
    written,
    [SynthPath::Voices(third, VoicePath::Gate), SynthPath::Sends(second)]
);
assert!(Index::<6>::new(6).is_none());
```

`Index<N>` can only be built in range, so a handle method or a path can never
name a missing element.

## Handles

`track` borrows the model and the change set together and returns the
handle; it lives as long as the caller keeps that borrow. The handle has one
method per field, named like the field, and three fixed ones:

- a leaf method returns a `Field`, with `get`, `set`, `set_if_changed`
  (needs `PartialEq`) and `update`; every write through it marks the leaf;
- a nested method returns the handle of the nested struct, so the same
  field names work at every level;
- `model()` reads the whole struct under the handle without marking anything;
- `set(value)` replaces the whole struct and marks every leaf under it;
- `mark_all()` marks every leaf under it without writing.

A function that only needs part of the model takes the narrower handle:

```rust
use fieldset::{Changes, Fields};

#[derive(Fields, Clone, Copy)]
pub struct Filter {
    pub cutoff: f32,
    pub resonance: f32,
}

#[derive(Fields)]
pub struct Preset {
    #[fields(nested)]
    pub filter: Filter,
    pub volume: f32,
}

fn open_up(mut filter: FilterMut<'_>, amount: f32) {
    let cutoff = filter.cutoff().get();
    filter.cutoff().set((cutoff + amount).min(1.0));
}

let mut preset = Preset { filter: Filter { cutoff: 0.2, resonance: 0.0 }, volume: 1.0 };
let mut changes = Changes::<Preset>::new();
open_up(changes.track(&mut preset).filter(), 0.5);
assert_eq!(preset.filter.cutoff, 0.7);
assert_eq!(changes.len(), 1);
```

## Paths

A path is a few bytes of plain data that names one leaf. The `Path` trait
gives `leaf`, the leaf index relative to its struct, and `from_leaf`, the
way back; the generated enum also has `from_leaf` as an inherent `const fn`.
Every nested field gets a projection on the path enum, named like the field,
that returns the inner path when the leaf lies under that field; for an
`indexed, nested` field the projection returns the element's `Index<N>`
together with the inner path.

```rust
use fieldset::{Fields, Path};

#[derive(Fields, Clone, Copy)]
pub struct Inner {
    pub a: u8,
    pub b: u8,
}

#[derive(Fields)]
pub struct Outer {
    pub x: u8,
    #[fields(nested)]
    pub inner: Inner,
}

let path = OuterPath::Inner(InnerPath::B);
assert_eq!(path.leaf(), 2);
assert_eq!(OuterPath::from_leaf(2), Some(path));
assert_eq!(OuterPath::from_leaf(3), None);
assert_eq!(path.inner(), Some(InnerPath::B));
assert_eq!(OuterPath::X.inner(), None);
```

A path carries no value. The reactor that receives it reads the model, which
already holds the final value; two writes to one leaf in one batch produce one
path.

## Changes

`Changes<X>` is a bitset with exactly `X::COUNT` bits, rounded up to whole
32-bit words, and nothing else. `new` is a `const fn`, so a set can live in a
`static` behind whatever cell the program already uses:

```rust
use std::sync::Mutex;
use fieldset::{Changes, Fields};

#[derive(Fields)]
pub struct State {
    pub level: u8,
}

static CHANGES: Mutex<Changes<State>> = Mutex::new(Changes::new());

let mut state = State { level: 0 };
CHANGES.lock().unwrap().track(&mut state).level().set(3);
assert_eq!(CHANGES.lock().unwrap().len(), 1);
```

- `drain()` yields each marked path in declaration order and clears it as it
  goes; dropping the iterator early leaves the rest marked for next time.
- `iter()` yields the same sequence without clearing.
- `mark_all()`, `clear()`, `len()` and `is_empty()` do what their names say.

Marking cannot fail and never allocates, so a write site needs no error
path and a batch of any size costs the same memory.

## Sizes

One bit per leaf: a model with 401 leaves tracks changes in 52 bytes. A path
is one byte for a struct of plain leaves, a few bytes when several nested
fields or an index are carried: the 401-leaf model above, three levels deep
with a six-element array, has a four-byte path and `Option` of it costs
nothing extra. The model's own size does not change.

## Rejected inputs

The derive reports each of these as a compile error on the offending struct
or field: tuple, unit, enum and union inputs; generic or lifetime parameters;
a `#[fields(..)]` attribute on the struct itself; a field with a default
value; `indexed` on a non-array or on an array of arrays; `skip` combined
with another key; unknown, duplicate or missing keys; zero non-skipped
fields; more than 256 non-skipped fields; more than 65535 leaves; a
non-skipped field named `model`, `set` or `mark_all`; a `nested` or
`indexed, nested` field named `leaf` or `from_leaf`; and two fields whose
names map to the same variant, such as `a_b` and `aB`.

## Compatibility

Stable Rust 1.88 or newer, `no_std` with no `alloc`, no `unsafe`. The crate
pair `fieldset` and `fieldset_macro` is released in lockstep; depend only on
`fieldset`.

## License

MIT.
