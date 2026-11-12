use core::cell::Cell;

use fieldset::{Changes, Fields, Index, Path};

#[derive(Fields)]
struct One {
    v: u8,
}

#[derive(Fields)]
struct Float {
    v: f32,
}

#[derive(Fields)]
struct Three {
    a: u8,
    b: u8,
    c: u8,
}

#[derive(Fields)]
#[allow(
    dead_code,
    reason = "only its leaf count matters; no model is ever built"
)]
struct ThirtyFive {
    f00: u8,
    f01: u8,
    f02: u8,
    f03: u8,
    f04: u8,
    f05: u8,
    f06: u8,
    f07: u8,
    f08: u8,
    f09: u8,
    f10: u8,
    f11: u8,
    f12: u8,
    f13: u8,
    f14: u8,
    f15: u8,
    f16: u8,
    f17: u8,
    f18: u8,
    f19: u8,
    f20: u8,
    f21: u8,
    f22: u8,
    f23: u8,
    f24: u8,
    f25: u8,
    f26: u8,
    f27: u8,
    f28: u8,
    f29: u8,
    f30: u8,
    f31: u8,
    f32: u8,
    f33: u8,
    f34: u8,
}

fn one(v: u8) -> (One, Changes<One>) {
    (One { v }, Changes::new())
}

fn three_marked() -> (Three, Changes<Three>) {
    let mut model = Three { a: 0, b: 0, c: 0 };
    let mut changes = Changes::<Three>::new();
    let mut handle = changes.track(&mut model);
    handle.a().set(1);
    handle.b().set(2);
    handle.c().set(3);
    (model, changes)
}

#[test]
fn get_reads_without_marking() {
    let (mut model, mut changes) = one(7);
    assert_eq!(changes.track(&mut model).v().get(), 7);
    assert!(changes.is_empty());
    assert_eq!(changes.drain().next(), None);
}

#[test]
fn set_if_changed_with_equal_value_marks_nothing() {
    let (mut model, mut changes) = one(7);
    changes.track(&mut model).v().set_if_changed(7);
    assert_eq!(model.v, 7);
    assert!(changes.is_empty());
}

#[test]
fn set_if_changed_with_different_value_marks() {
    let (mut model, mut changes) = one(7);
    changes.track(&mut model).v().set_if_changed(8);
    assert_eq!(model.v, 8);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes.drain().next(), Some(OnePath::V));
}

#[test]
fn set_if_changed_with_nan_always_marks() {
    let mut model = Float { v: f32::NAN };
    let mut changes = Changes::<Float>::new();
    changes.track(&mut model).v().set_if_changed(f32::NAN);
    assert!(model.v.is_nan());
    assert_eq!(changes.drain().next(), Some(FloatPath::V));
}

#[test]
fn update_mutates_in_place_and_marks() {
    let (mut model, mut changes) = one(7);
    changes.track(&mut model).v().update(|v| *v += 1);
    assert_eq!(model.v, 8);
    assert_eq!(changes.drain().next(), Some(OnePath::V));
}

#[test]
fn update_marks_even_when_nothing_changes() {
    let (mut model, mut changes) = one(7);
    changes.track(&mut model).v().update(|_| {});
    assert_eq!(model.v, 7);
    assert_eq!(changes.len(), 1);
}

#[test]
fn field_is_reusable_for_its_borrow() {
    let (mut model, mut changes) = one(0);
    let mut handle = changes.track(&mut model);
    let mut field = handle.v();
    field.set(3);
    assert_eq!(field.get(), 3);
    field.update(|v| *v *= 2);
    assert_eq!(field.get(), 6);
    assert_eq!(model.v, 6);
}

#[test]
fn iter_is_ordered_and_non_destructive() {
    let mut model = Three { a: 0, b: 0, c: 0 };
    let mut changes = Changes::<Three>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.c().set(1);
        handle.a().set(1);
    }
    let seen: Vec<_> = changes.iter().collect();
    assert_eq!(seen, [ThreePath::A, ThreePath::C]);
    assert_eq!(changes.len(), 2);
    let drained: Vec<_> = changes.drain().collect();
    assert_eq!(drained, seen);
    assert!(changes.is_empty());
}

#[test]
fn iter_on_empty_set_yields_nothing() {
    let changes = Changes::<Three>::new();
    assert_eq!(changes.iter().next(), None);
}

#[test]
fn mark_all_sets_exactly_count_bits() {
    assert_eq!(<ThirtyFive as Fields>::COUNT, 35);
    assert_eq!(
        core::mem::size_of::<<ThirtyFive as Fields>::Words>(),
        2 * core::mem::size_of::<u32>()
    );
    let mut changes = Changes::<ThirtyFive>::new();
    changes.mark_all();
    assert_eq!(changes.len(), 35);
    assert!(!changes.is_empty());
    assert_eq!(changes.iter().count(), 35);
    let drained: Vec<_> = changes.drain().collect();
    assert_eq!(drained.len(), 35);
    assert_eq!(drained.first(), Some(&ThirtyFivePath::F00));
    assert_eq!(drained.last(), Some(&ThirtyFivePath::F34));
    for (leaf, path) in drained.iter().enumerate() {
        assert_eq!(u16::try_from(leaf).ok(), Some(path.leaf()));
    }
    assert!(changes.is_empty());
}

#[test]
fn mark_all_is_idempotent() {
    let mut changes = Changes::<ThirtyFive>::new();
    changes.mark_all();
    changes.mark_all();
    assert_eq!(changes.len(), 35);
}

#[test]
fn clear_empties_the_set() {
    let (_, mut changes) = three_marked();
    assert_eq!(changes.len(), 3);
    changes.clear();
    assert!(changes.is_empty());
    assert_eq!(changes.len(), 0);
    assert_eq!(changes.drain().next(), None);
}

#[test]
fn clear_on_empty_set_is_a_no_op() {
    let mut changes = Changes::<Three>::new();
    changes.clear();
    assert!(changes.is_empty());
}

#[test]
fn len_counts_marked_leaves() {
    let mut model = Three { a: 0, b: 0, c: 0 };
    let mut changes = Changes::<Three>::new();
    assert_eq!(changes.len(), 0);
    changes.track(&mut model).b().set(1);
    assert_eq!(changes.len(), 1);
    changes.track(&mut model).b().set(2);
    assert_eq!(changes.len(), 1);
    changes.track(&mut model).a().set(1);
    assert_eq!(changes.len(), 2);
}

#[test]
fn dropped_partial_drain_keeps_unvisited_leaves() {
    let (_, mut changes) = three_marked();
    {
        let mut drain = changes.drain();
        assert_eq!(drain.next(), Some(ThreePath::A));
    }
    assert_eq!(changes.len(), 2);
    assert_eq!(
        changes.iter().collect::<Vec<_>>(),
        [ThreePath::B, ThreePath::C]
    );
    let mut drain = changes.drain();
    assert_eq!(drain.next(), Some(ThreePath::B));
    assert_eq!(drain.next(), Some(ThreePath::C));
    assert_eq!(drain.next(), None);
}

#[test]
fn index_within_bounds_is_some() {
    let index = Index::<4>::new(3);
    assert_eq!(index.map(Index::get), Some(3));
    assert_eq!(Index::<4>::new(0).map(Index::get), Some(0));
}

#[test]
fn index_at_or_beyond_bound_is_none() {
    assert_eq!(Index::<4>::new(4), None);
    assert_eq!(Index::<4>::new(5), None);
}

#[test]
fn index_is_const_and_ordered() {
    const FIRST: Option<Index<4>> = Index::new(1);
    const SECOND: Option<Index<4>> = Index::new(2);
    assert!(FIRST < SECOND);
    assert_eq!(FIRST, Index::new(1));
    assert_eq!(
        core::mem::size_of::<Index<4>>(),
        core::mem::size_of::<u16>()
    );
}

struct WithCell {
    cell: Cell<u8>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum WithCellPath {
    Cell,
}

impl Path for WithCellPath {
    fn leaf(self) -> u16 {
        0
    }

    fn from_leaf(leaf: u16) -> Option<Self> {
        (leaf == 0).then_some(Self::Cell)
    }
}

struct WithCellMut<'a> {
    _model: &'a mut WithCell,
    _mask: &'a mut fieldset::Mask,
    _base: u16,
}

impl Fields for WithCell {
    const COUNT: usize = 1;
    type Words = [u32; 1];
    type Path = WithCellPath;
    type Mut<'a> = WithCellMut<'a>;

    fn handle<'a>(&'a mut self, mask: &'a mut fieldset::Mask, base: u16) -> WithCellMut<'a> {
        WithCellMut {
            _model: self,
            _mask: mask,
            _base: base,
        }
    }
}

static CELL_CHANGES: Changes<WithCell> = Changes::new();

#[test]
fn changes_is_send_and_sync_whatever_the_model_is() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Changes<WithCell>>();
    assert!(CELL_CHANGES.is_empty());
    assert_eq!(CELL_CHANGES.len(), 0);
    assert_eq!(CELL_CHANGES.iter().next(), None);

    let mut model = WithCell { cell: Cell::new(3) };
    let mut changes = Changes::<WithCell>::new();
    {
        let _handle = changes.track(&mut model);
    }
    assert!(changes.is_empty());
    assert_eq!(model.cell.get(), 3);
}
