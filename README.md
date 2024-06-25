# fieldset

![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![CI](https://github.com/liarokapisv/fieldset/actions/workflows/ci.yml/badge.svg)](https://github.com/liarokapisv/fieldset/actions)
[![docs](https://docs.rs/fieldset/badge.svg)](https://docs.rs/fieldset)

This library tracks field modifications as data. It is intended to provide a bounded-space alternative to
event listeners, designed for but not restricted to usage in embedded systems.

It works by deriving a `FieldType`, a `FieldSetter` trait and multiple `FieldSet` types from a struct.
- The `FieldType` is an enum where each variant corresponds to each field of the structure.
- The `FieldSetter` trait consists of one setter method for each field.
- The `FieldSet` types implement the `FieldSetter` trait and provide an iterator interface where each item is a `FieldType` instance corresponding to modified fields.

Subsystems can use the `FieldSetter` interface to "modify" fields of the original parameter structure
and then the `FieldSet` can be iterated upon to notify other subsystems that the fields were modified.
This allows for implementing event-driven architectures by batching modifications and then notifying afterwards.

There are multiple `FieldSetter` implementations with different tradeoffs regarding iteration and backup storage.

- `OptFieldSet` is backed by a derived struct where each field is converted to an `Option`. Each iteration goes through all fields and is therefore suitable for smaller structures or frequent modifications.
- `BitFieldSet` is backed by an iteration array of `FieldType` with length equal to the number of fields, and a `bitfield` that tracks which fields have been modified. Iteration is optimal and only goes through exactly as many fields as were modified. Has the drawback that each field can only be modified once before iteration and subsequent modifications are ignored. This is often a good compromise.
- `PerfFieldSet` is backed by an array of `FieldType` of length equal to the number of fields and a complementary array that tracks which fields have been modified and their current position in the iteration array. Iteration is optimal and only goes through exactly as many fields as were modified. Fields can be modified multiple times and only the latest modification applies. Has the drawback of the extra space needed to track the multiple modifications.

The library currently requires the usage of the nightly `impl_trait_in_assoc_type` feature.

## Example

```rust
#![feature(impl_trait_in_assoc_type)]
use fieldset::{FieldSetter, FieldSet};

#[derive(Default, FieldSet)]
struct SubModel {
    a: f32,
    b: u32
}

#[derive(Default, FieldSet)]
struct DomainModel {
    #[fieldset]
    sub: SubModel,
    c: f32
}

fn sub_modifier(mut model: impl SubModelFieldSetter, i: u32) {
    model.b().set(i);
}

fn modifier(mut model: impl DomainModelFieldSetter, i: u32) {
    model.c().set(i as f32);
    sub_modifier(model.sub(), i);
}

fn example() {
    let mut model = DomainModel::default();
    for i in 0..10 {
        let mut field_set = DomainModelPerfFieldSet::default();
        modifier(&mut field_set, i);
        let mut iter = field_set.into_iter();
        for field_change in iter.clone() {
            model.apply(field_change);
        }
        assert_eq!(iter.next(), Some(DomainModelFieldType::C(i as f32)));
        assert_eq!(iter.next(), Some(DomainModelFieldType::Sub(SubModelFieldType::B(i))));
        assert_eq!(model.c, i as f32);
        assert_eq!(model.sub.b, i);
    }
}
```
