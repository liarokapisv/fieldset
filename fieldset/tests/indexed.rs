use fieldset::{Changes, Fields, Index, Path};

#[derive(Fields, Debug, PartialEq, Clone, Copy)]
struct S {
    x: u8,
    y: u8,
}

#[derive(Fields, Debug, PartialEq)]
struct Root {
    a: u8,
    #[fields(indexed)]
    v: [u8; 4],
    #[fields(indexed, nested)]
    e: [S; 3],
    z: u8,
}

const VOICES: usize = 6;

#[derive(Fields)]
struct Voiced {
    #[fields(indexed)]
    v: [u8; VOICES],
    #[fields(indexed, nested)]
    e: [S; VOICES],
}

#[derive(Fields)]
struct PlainArray {
    w: [u8; 3],
}

#[derive(Fields)]
struct OnlyIndexed {
    #[fields(indexed)]
    v: [u16; 2],
}

fn root() -> Root {
    Root {
        a: 0,
        v: [0; 4],
        e: [S { x: 0, y: 0 }; 3],
        z: 0,
    }
}

fn index<const N: usize>(i: usize) -> Index<N> {
    Index::new(i).unwrap_or_else(|| panic!("{i} is out of range for {N}"))
}

fn leaf_index(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

fn drained<R: Fields>(changes: &mut Changes<R>) -> Vec<R::Path> {
    changes.drain().collect()
}

fn root_leaves() -> Vec<RootPath> {
    let mut leaves = vec![RootPath::A];
    leaves.extend((0..4).map(|i| RootPath::V(index(i))));
    for i in 0..3 {
        leaves.push(RootPath::E(index(i), SPath::X));
        leaves.push(RootPath::E(index(i), SPath::Y));
    }
    leaves.push(RootPath::Z);
    leaves
}

#[test]
fn counts() {
    assert_eq!(<S as Fields>::COUNT, 2);
    assert_eq!(<Root as Fields>::COUNT, 1 + 4 + 3 * 2 + 1);
    assert_eq!(<Voiced as Fields>::COUNT, VOICES + VOICES * 2);
    assert_eq!(<PlainArray as Fields>::COUNT, 1);
    assert_eq!(<OnlyIndexed as Fields>::COUNT, 2);
    assert_eq!(core::mem::size_of::<<Root as Fields>::Words>(), 4);
}

#[test]
fn numbering_and_round_trip() {
    let leaves = root_leaves();
    assert_eq!(leaves.len(), <Root as Fields>::COUNT);
    for (index, expected) in leaves.iter().enumerate() {
        let index = leaf_index(index);
        assert_eq!(Path::leaf(*expected), index);
        assert_eq!(<RootPath as Path>::from_leaf(index), Some(*expected));
        assert_eq!(RootPath::from_leaf(index), Some(*expected));
    }
    assert_eq!(
        <RootPath as Path>::from_leaf(leaf_index(<Root as Fields>::COUNT)),
        None
    );
    assert_eq!(<RootPath as Path>::from_leaf(u16::MAX), None);
}

#[test]
fn from_leaf_is_const() {
    const THIRD_ELEMENT: Option<RootPath> = RootPath::from_leaf(3);
    const LAST_Y: Option<RootPath> = RootPath::from_leaf(10);
    const PAST_END: Option<RootPath> = RootPath::from_leaf(12);
    assert_eq!(THIRD_ELEMENT, Some(RootPath::V(index(2))));
    assert_eq!(LAST_Y, Some(RootPath::E(index(2), SPath::Y)));
    assert_eq!(PAST_END, None);
}

#[test]
fn indexed_leaves_follow_the_first_leaf() {
    for i in 0..4 {
        let mut model = root();
        let mut changes = Changes::<Root>::new();
        changes.track(&mut model).v(index(i)).set(7);
        assert_eq!(changes.len(), 1);
        assert_eq!(drained(&mut changes), vec![RootPath::V(index(i))]);
        assert_eq!(Path::leaf(RootPath::V(index::<4>(i))), leaf_index(1 + i));
        let mut expected = [0; 4];
        expected[i] = 7;
        assert_eq!(model.v, expected);
    }
}

#[test]
fn indexed_nested_leaves_are_per_element() {
    for i in 0..3 {
        let mut model = root();
        let mut changes = Changes::<Root>::new();
        changes.track(&mut model).e(index(i)).x().set(5);
        assert_eq!(drained(&mut changes), vec![RootPath::E(index(i), SPath::X)]);
        assert_eq!(
            Path::leaf(RootPath::E(index::<3>(i), SPath::X)),
            leaf_index(5 + i * 2)
        );
        assert_eq!(
            Path::leaf(RootPath::E(index::<3>(i), SPath::Y)),
            leaf_index(5 + i * 2 + 1)
        );
        assert_eq!(model.e[i].x, 5);
        assert_eq!(model.e[i].y, 0);
    }
}

#[test]
fn writes_drain_in_declaration_order() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.z().set(9);
        handle.e(index(2)).y().set(8);
        handle.e(index(0)).x().set(7);
        handle.v(index(3)).set(6);
        handle.v(index(1)).set(5);
        handle.a().set(4);
    }
    assert_eq!(
        drained(&mut changes),
        vec![
            RootPath::A,
            RootPath::V(index(1)),
            RootPath::V(index(3)),
            RootPath::E(index(0), SPath::X),
            RootPath::E(index(2), SPath::Y),
            RootPath::Z,
        ]
    );
    assert_eq!(
        model,
        Root {
            a: 4,
            v: [0, 5, 0, 6],
            e: [S { x: 7, y: 0 }, S { x: 0, y: 0 }, S { x: 0, y: 8 }],
            z: 9,
        }
    );
}

#[test]
fn every_leaf_through_its_handle() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.a().set(1);
        for i in 0..4 {
            handle.v(index(i)).set(1);
        }
        for i in 0..3 {
            handle.e(index(i)).x().set(1);
            handle.e(index(i)).y().set(1);
        }
        handle.z().set(1);
    }
    assert_eq!(changes.len(), <Root as Fields>::COUNT);
    assert_eq!(drained(&mut changes), root_leaves());
}

#[test]
fn projection() {
    for i in 0..3 {
        assert_eq!(
            RootPath::E(index(i), SPath::Y).e(),
            Some((index::<3>(i), SPath::Y))
        );
    }
    assert_eq!(RootPath::A.e(), None);
    assert_eq!(RootPath::V(index(0)).e(), None);
    assert_eq!(RootPath::Z.e(), None);
    assert_eq!(
        RootPath::E(index(1), SPath::X)
            .e()
            .map(|(i, inner)| (Index::get(i), inner)),
        Some((1, SPath::X))
    );
}

#[test]
fn element_set_marks_exactly_that_element() {
    for i in 0..3 {
        let mut model = root();
        let mut changes = Changes::<Root>::new();
        changes.track(&mut model).e(index(i)).set(S { x: 3, y: 4 });
        let paths = drained(&mut changes);
        assert_eq!(paths.len(), <S as Fields>::COUNT);
        assert_eq!(
            paths,
            vec![
                RootPath::E(index(i), SPath::X),
                RootPath::E(index(i), SPath::Y),
            ]
        );
        assert!(paths.iter().all(|path| matches!(
            path,
            RootPath::E(j, _) if Index::get(*j) == i
        )));
        assert_eq!(model.e[i], S { x: 3, y: 4 });
        for (j, element) in model.e.iter().enumerate() {
            if j != i {
                assert_eq!(*element, S { x: 0, y: 0 });
            }
        }
    }
}

#[test]
fn element_mark_all_marks_exactly_that_element() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).e(index(1)).mark_all();
    assert_eq!(
        drained(&mut changes),
        vec![
            RootPath::E(index(1), SPath::X),
            RootPath::E(index(1), SPath::Y),
        ]
    );
    assert_eq!(model, root());
}

#[test]
fn root_set_and_mark_all_cover_every_leaf() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mark_all();
    assert_eq!(drained(&mut changes), root_leaves());
    changes.track(&mut model).set(Root {
        a: 1,
        v: [2, 3, 4, 5],
        e: [S { x: 6, y: 7 }; 3],
        z: 8,
    });
    assert_eq!(drained(&mut changes), root_leaves());
    assert_eq!(model.v, [2, 3, 4, 5]);
}

#[test]
fn field_reads_and_set_if_changed() {
    let mut model = root();
    model.v[2] = 9;
    let mut changes = Changes::<Root>::new();
    {
        let mut handle = changes.track(&mut model);
        assert_eq!(handle.v(index(2)).get(), 9);
        handle.v(index(2)).set_if_changed(9);
        assert_eq!(handle.e(index(1)).model().x, 0);
    }
    assert!(changes.is_empty());
}

#[test]
fn const_length() {
    let mut model = Voiced {
        v: [0; VOICES],
        e: [S { x: 0, y: 0 }; VOICES],
    };
    let mut changes = Changes::<Voiced>::new();
    {
        let mut handle = changes.track(&mut model);
        let last: Index<VOICES> = index(VOICES - 1);
        handle.v(last).set(1);
        handle.e(last).y().set(2);
    }
    assert_eq!(
        drained(&mut changes),
        vec![
            VoicedPath::V(index(VOICES - 1)),
            VoicedPath::E(index(VOICES - 1), SPath::Y),
        ]
    );
    assert_eq!(
        Path::leaf(VoicedPath::E(index(VOICES - 1), SPath::Y)),
        leaf_index(VOICES + (VOICES - 1) * 2 + 1)
    );
    assert_eq!(
        <VoicedPath as Path>::from_leaf(leaf_index(<Voiced as Fields>::COUNT)),
        None
    );
    assert_eq!(model.v[VOICES - 1], 1);
    assert_eq!(model.e[VOICES - 1].y, 2);
}

#[test]
fn plain_array_stays_one_leaf() {
    let mut model = PlainArray { w: [0; 3] };
    let mut changes = Changes::<PlainArray>::new();
    changes.track(&mut model).w().update(|w| w[1] = 1);
    assert_eq!(drained(&mut changes), vec![PlainArrayPath::W]);
    assert_eq!(model.w, [0, 1, 0]);
}

#[test]
fn only_indexed() {
    let mut model = OnlyIndexed { v: [0; 2] };
    let mut changes = Changes::<OnlyIndexed>::new();
    changes.track(&mut model).v(index(1)).set(3);
    assert_eq!(drained(&mut changes), vec![OnlyIndexedPath::V(index(1))]);
    assert_eq!(
        OnlyIndexedPath::from_leaf(1),
        Some(OnlyIndexedPath::V(index(1)))
    );
    assert_eq!(OnlyIndexedPath::from_leaf(2), None);
}

#[test]
fn path_sizes() {
    assert_eq!(core::mem::size_of::<Index<4>>(), 2);
    assert!(core::mem::size_of::<OnlyIndexedPath>() <= 4);
    assert!(core::mem::size_of::<RootPath>() <= 4);
    assert!(core::mem::size_of::<Option<RootPath>>() <= 4);
}
