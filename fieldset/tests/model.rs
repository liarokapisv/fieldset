use std::collections::BTreeSet;

use fieldset::{Changes, Field, Fields, Index, Path};
use proptest::prelude::*;

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
struct Low {
    c: u8,
    d: u8,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
struct Mid {
    b: u8,
    #[fields(nested)]
    low: Low,
}

#[derive(Clone, Copy, Debug)]
struct NoEq(u8);

#[derive(Fields, Clone, Copy, Debug)]
struct Model {
    a: u8,
    #[fields(nested)]
    mid: Mid,
    #[fields(indexed)]
    v: [u8; 3],
    #[fields(indexed, nested)]
    e: [Low; 2],
    #[fields(skip)]
    meta: u32,
    f: NoEq,
}

const COUNT: usize = 12;

const fn idx<const N: usize>(i: usize) -> Index<N> {
    match Index::new(i) {
        Some(index) => index,
        None => panic!("index out of range"),
    }
}

// Every leaf of `Model` in declaration order, written out by hand.
const PATHS: [ModelPath; COUNT] = [
    ModelPath::A,
    ModelPath::Mid(MidPath::B),
    ModelPath::Mid(MidPath::Low(LowPath::C)),
    ModelPath::Mid(MidPath::Low(LowPath::D)),
    ModelPath::V(idx(0)),
    ModelPath::V(idx(1)),
    ModelPath::V(idx(2)),
    ModelPath::E(idx(0), LowPath::C),
    ModelPath::E(idx(0), LowPath::D),
    ModelPath::E(idx(1), LowPath::C),
    ModelPath::E(idx(1), LowPath::D),
    ModelPath::F,
];

// First leaf of each field, by hand: `a` is one leaf, `mid` is three
// (`b`, `low.c`, `low.d`), `v` is three, `e` is two elements of two, `meta`
// is skipped and `f` is last.
const A: u16 = 0;
const MID: u16 = 1;
const MID_LOW: u16 = 2;
const V: u16 = 4;
const E: u16 = 7;
const LOW_COUNT: u16 = 2;
const F: u16 = 11;
const END: u16 = 12;

#[derive(Clone, Copy, Debug)]
enum Leaf {
    A,
    B,
    C,
    D,
    V(usize),
    Ec(usize),
    Ed(usize),
    F,
}

impl Leaf {
    fn index(self) -> u16 {
        match self {
            Leaf::A => A,
            Leaf::B => MID,
            Leaf::C => MID_LOW,
            Leaf::D => MID_LOW + 1,
            Leaf::V(i) => V + small(i),
            Leaf::Ec(i) => E + small(i) * LOW_COUNT,
            Leaf::Ed(i) => E + small(i) * LOW_COUNT + 1,
            Leaf::F => F,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Subtree {
    Root,
    Mid,
    MidLow,
    E(usize),
}

impl Subtree {
    fn range(self) -> core::ops::Range<u16> {
        match self {
            Subtree::Root => 0..END,
            Subtree::Mid => MID..V,
            Subtree::MidLow => MID_LOW..V,
            Subtree::E(i) => {
                let start = E + small(i) * LOW_COUNT;
                start..start + LOW_COUNT
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Set(Leaf, u8),
    // `None` writes the value the leaf already holds.
    SetIfChanged(Leaf, Option<u8>),
    Update(Leaf, u8),
    SetSubtree(Subtree, u8),
    MarkAll(Subtree),
    Drain,
    DrainFirst(usize),
    Iter,
    Clear,
}

fn small(i: usize) -> u16 {
    u16::try_from(i).unwrap_or(u16::MAX)
}

fn model() -> Model {
    Model {
        a: 0,
        mid: Mid {
            b: 0,
            low: Low { c: 0, d: 0 },
        },
        v: [0; 3],
        e: [Low { c: 0, d: 0 }; 2],
        meta: 0xDEAD_BEEF,
        f: NoEq(0),
    }
}

fn values(model: &Model) -> [u8; COUNT] {
    [
        model.a,
        model.mid.b,
        model.mid.low.c,
        model.mid.low.d,
        model.v[0],
        model.v[1],
        model.v[2],
        model.e[0].c,
        model.e[0].d,
        model.e[1].c,
        model.e[1].d,
        model.f.0,
    ]
}

fn read(model: &Model, leaf: Leaf) -> u8 {
    values(model)[usize::from(leaf.index())]
}

fn leaf_mut(model: &mut Model, leaf: Leaf) -> &mut u8 {
    match leaf {
        Leaf::A => &mut model.a,
        Leaf::B => &mut model.mid.b,
        Leaf::C => &mut model.mid.low.c,
        Leaf::D => &mut model.mid.low.d,
        Leaf::V(i) => &mut model.v[i],
        Leaf::Ec(i) => &mut model.e[i].c,
        Leaf::Ed(i) => &mut model.e[i].d,
        Leaf::F => &mut model.f.0,
    }
}

// Runs `f` on the `u8` field for `leaf`; `None` for `f`, whose type has no
// `PartialEq`.
fn with_u8<R>(
    handle: &mut ModelMut<'_>,
    leaf: Leaf,
    f: impl FnOnce(&mut Field<'_, u8>) -> R,
) -> Option<R> {
    Some(match leaf {
        Leaf::A => f(&mut handle.a()),
        Leaf::B => f(&mut handle.mid().b()),
        Leaf::C => f(&mut handle.mid().low().c()),
        Leaf::D => f(&mut handle.mid().low().d()),
        Leaf::V(i) => f(&mut handle.v(idx(i))),
        Leaf::Ec(i) => f(&mut handle.e(idx(i)).c()),
        Leaf::Ed(i) => f(&mut handle.e(idx(i)).d()),
        Leaf::F => return None,
    })
}

fn leaf() -> impl Strategy<Value = Leaf> {
    prop_oneof![
        Just(Leaf::A),
        Just(Leaf::B),
        Just(Leaf::C),
        Just(Leaf::D),
        (0..3usize).prop_map(Leaf::V),
        (0..2usize).prop_map(Leaf::Ec),
        (0..2usize).prop_map(Leaf::Ed),
        Just(Leaf::F),
    ]
}

fn u8_leaf() -> impl Strategy<Value = Leaf> {
    leaf().prop_filter("set_if_changed needs PartialEq", |leaf| {
        !matches!(leaf, Leaf::F)
    })
}

fn subtree() -> impl Strategy<Value = Subtree> {
    prop_oneof![
        Just(Subtree::Root),
        Just(Subtree::Mid),
        Just(Subtree::MidLow),
        (0..2usize).prop_map(Subtree::E),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (leaf(), any::<u8>()).prop_map(|(leaf, value)| Op::Set(leaf, value)),
        4 => (u8_leaf(), proptest::option::of(any::<u8>()))
            .prop_map(|(leaf, value)| Op::SetIfChanged(leaf, value)),
        2 => (leaf(), any::<u8>()).prop_map(|(leaf, delta)| Op::Update(leaf, delta)),
        2 => (subtree(), any::<u8>()).prop_map(|(subtree, value)| Op::SetSubtree(subtree, value)),
        2 => subtree().prop_map(Op::MarkAll),
        2 => Just(Op::Drain),
        2 => (0..=COUNT).prop_map(Op::DrainFirst),
        1 => Just(Op::Iter),
        1 => Just(Op::Clear),
    ]
}

fn expected_paths(leaves: impl IntoIterator<Item = u16>) -> Vec<ModelPath> {
    leaves
        .into_iter()
        .map(|leaf| PATHS[usize::from(leaf)])
        .collect()
}

fn check(
    changes: &Changes<Model>,
    reference: &BTreeSet<u16>,
    model: &Model,
    expected: &Model,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        changes.iter().collect::<Vec<_>>(),
        expected_paths(reference.iter().copied())
    );
    prop_assert_eq!(changes.len(), reference.len());
    prop_assert_eq!(changes.is_empty(), reference.is_empty());
    prop_assert_eq!(values(model), values(expected));
    prop_assert_eq!(model.meta, expected.meta);
    Ok(())
}

fn apply(
    op: Op,
    changes: &mut Changes<Model>,
    reference: &mut BTreeSet<u16>,
    model: &mut Model,
    expected: &mut Model,
) -> Result<(), TestCaseError> {
    match op {
        Op::Set(leaf, value) => {
            let mut handle = changes.track(model);
            if with_u8(&mut handle, leaf, |field| field.set(value)).is_none() {
                handle.f().set(NoEq(value));
            }
            *leaf_mut(expected, leaf) = value;
            reference.insert(leaf.index());
        }
        Op::SetIfChanged(leaf, value) => {
            let current = read(model, leaf);
            let value = value.unwrap_or(current);
            let mut handle = changes.track(model);
            let written = with_u8(&mut handle, leaf, |field| field.set_if_changed(value));
            prop_assert!(written.is_some(), "{leaf:?} has no set_if_changed");
            *leaf_mut(expected, leaf) = value;
            if value != current {
                reference.insert(leaf.index());
            }
        }
        Op::Update(leaf, delta) => {
            let mut handle = changes.track(model);
            let bump = |x: &mut u8| *x = x.wrapping_add(delta);
            if with_u8(&mut handle, leaf, |field| field.update(bump)).is_none() {
                handle.f().update(|f| bump(&mut f.0));
            }
            let slot = leaf_mut(expected, leaf);
            *slot = slot.wrapping_add(delta);
            reference.insert(leaf.index());
        }
        Op::SetSubtree(subtree, value) => {
            let low = Low { c: value, d: value };
            let mut handle = changes.track(model);
            match subtree {
                Subtree::Root => {
                    let whole = Model {
                        a: value,
                        mid: Mid { b: value, low },
                        v: [value; 3],
                        e: [low; 2],
                        meta: u32::from(value),
                        f: NoEq(value),
                    };
                    handle.set(whole);
                    *expected = whole;
                }
                Subtree::Mid => {
                    handle.mid().set(Mid { b: value, low });
                    expected.mid = Mid { b: value, low };
                }
                Subtree::MidLow => {
                    handle.mid().low().set(low);
                    expected.mid.low = low;
                }
                Subtree::E(i) => {
                    handle.e(idx(i)).set(low);
                    expected.e[i] = low;
                }
            }
            reference.extend(subtree.range());
        }
        Op::MarkAll(subtree) => {
            let mut handle = changes.track(model);
            match subtree {
                Subtree::Root => handle.mark_all(),
                Subtree::Mid => handle.mid().mark_all(),
                Subtree::MidLow => handle.mid().low().mark_all(),
                Subtree::E(i) => handle.e(idx(i)).mark_all(),
            }
            reference.extend(subtree.range());
        }
        Op::Drain => {
            let drained: Vec<ModelPath> = changes.drain().collect();
            prop_assert_eq!(drained, expected_paths(reference.iter().copied()));
            reference.clear();
        }
        Op::DrainFirst(k) => {
            let drained: Vec<ModelPath> = changes.drain().take(k).collect();
            let first: Vec<u16> = reference.iter().copied().take(k).collect();
            prop_assert_eq!(drained, expected_paths(first.iter().copied()));
            for leaf in first {
                reference.remove(&leaf);
            }
        }
        Op::Iter => {
            prop_assert_eq!(changes.iter().count(), reference.len());
        }
        Op::Clear => {
            changes.clear();
            reference.clear();
        }
    }
    check(changes, reference, model, expected)
}

fn run(ops: &[Op]) -> Result<(), TestCaseError> {
    let mut changes = Changes::<Model>::new();
    let mut reference = BTreeSet::new();
    let mut model = model();
    let mut expected = model;
    check(&changes, &reference, &model, &expected)?;
    for &op in ops {
        apply(op, &mut changes, &mut reference, &mut model, &mut expected)?;
    }
    Ok(())
}

#[test]
fn shape() {
    assert_eq!(<Model as Fields>::COUNT, COUNT);
    assert_eq!(<Mid as Fields>::COUNT, 3);
    assert_eq!(<Low as Fields>::COUNT, usize::from(LOW_COUNT));
    assert_eq!(core::mem::size_of::<Changes<Model>>(), 4);
    assert!(core::mem::size_of::<ModelPath>() <= 4);
}

#[test]
fn hand_written_paths_are_the_leaves() {
    for (i, path) in PATHS.iter().enumerate() {
        let leaf = small(i);
        assert_eq!(Path::leaf(*path), leaf);
        assert_eq!(<ModelPath as Path>::from_leaf(leaf), Some(*path));
    }
    assert_eq!(<ModelPath as Path>::from_leaf(END), None);
}

#[test]
fn every_leaf_has_an_index_in_the_list() {
    let all = [
        Leaf::A,
        Leaf::B,
        Leaf::C,
        Leaf::D,
        Leaf::V(0),
        Leaf::V(1),
        Leaf::V(2),
        Leaf::Ec(0),
        Leaf::Ed(0),
        Leaf::Ec(1),
        Leaf::Ed(1),
        Leaf::F,
    ];
    let indices: Vec<u16> = all.iter().map(|leaf| leaf.index()).collect();
    assert_eq!(indices, (0..END).collect::<Vec<_>>());
    assert_eq!(Subtree::Root.range().len(), COUNT);
    assert_eq!(Subtree::Mid.range(), 1..4);
    assert_eq!(Subtree::MidLow.range(), 2..4);
    assert_eq!(Subtree::E(0).range(), 7..9);
    assert_eq!(Subtree::E(1).range(), 9..11);
}

#[test]
fn equal_value_write_marks_nothing() {
    let mut model = model();
    model.mid.low.d = 9;
    let mut changes = Changes::<Model>::new();
    changes.track(&mut model).mid().low().d().set_if_changed(9);
    assert!(changes.is_empty());
    assert_eq!(model.mid.low.d, 9);
    changes.track(&mut model).mid().low().d().set_if_changed(10);
    assert_eq!(
        changes.drain().collect::<Vec<_>>(),
        vec![ModelPath::Mid(MidPath::Low(LowPath::D))]
    );
    assert_eq!(model.mid.low.d, 10);
}

#[test]
fn non_partial_eq_leaf_marks_on_set_and_update() {
    let mut model = model();
    let mut changes = Changes::<Model>::new();
    changes.track(&mut model).f().set(NoEq(3));
    assert_eq!(changes.drain().collect::<Vec<_>>(), vec![ModelPath::F]);
    changes.track(&mut model).f().update(|f| f.0 += 1);
    assert_eq!(changes.drain().collect::<Vec<_>>(), vec![ModelPath::F]);
    assert_eq!(model.f.0, 4);
    assert_eq!(model.meta, 0xDEAD_BEEF);
}

proptest! {
    #[test]
    fn changes_match_the_reference_model(ops in proptest::collection::vec(op(), 0..=64)) {
        run(&ops)?;
    }
}
