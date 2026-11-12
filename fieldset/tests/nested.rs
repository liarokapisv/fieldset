use fieldset::{Changes, Fields, Path};

mod inner {
    use fieldset::Fields;

    #[derive(Fields, Debug, PartialEq, Clone, Copy)]
    pub struct Leaf {
        pub x: u8,
        pub y: u8,
    }

    #[derive(Fields, Debug, PartialEq, Clone, Copy)]
    pub struct Mid {
        #[fields(nested)]
        pub child: Leaf,
        pub m: u8,
    }
}

use inner::{Leaf, LeafPath, Mid, MidPath};

#[derive(Fields, Debug, PartialEq)]
struct Root {
    a: u8,
    #[fields(nested)]
    mid: inner::Mid,
    z: u8,
    #[fields(skip)]
    meta: u32,
}

type MidAlias = Mid;

#[derive(Fields)]
struct Aliased {
    #[fields(nested)]
    mid: MidAlias,
    #[fields(skip)]
    #[allow(dead_code)]
    meta: u32,
}

#[derive(Fields)]
struct Other {
    pad: u8,
    #[fields(nested)]
    one: Leaf,
    #[fields(nested)]
    mid: Mid,
}

#[derive(Fields)]
struct PathNamed {
    leaf: u8,
    from_leaf: u8,
}

fn root() -> Root {
    Root {
        a: 0,
        mid: Mid {
            child: Leaf { x: 0, y: 0 },
            m: 0,
        },
        z: 0,
        meta: 0,
    }
}

const ROOT_LEAVES: [RootPath; 5] = [
    RootPath::A,
    RootPath::Mid(MidPath::Child(LeafPath::X)),
    RootPath::Mid(MidPath::Child(LeafPath::Y)),
    RootPath::Mid(MidPath::M),
    RootPath::Z,
];

fn leaf_index(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

fn drained<R: Fields>(changes: &mut Changes<R>) -> Vec<R::Path> {
    changes.drain().collect()
}

#[test]
fn three_levels_count_and_round_trip() {
    assert_eq!(<Leaf as Fields>::COUNT, 2);
    assert_eq!(<Mid as Fields>::COUNT, 3);
    assert_eq!(<Root as Fields>::COUNT, 5);
    for (index, expected) in ROOT_LEAVES.iter().enumerate() {
        let index = leaf_index(index);
        let path = <RootPath as Path>::from_leaf(index);
        assert_eq!(path, Some(*expected));
        assert_eq!(path.map(Path::leaf), Some(index));
        assert_eq!(RootPath::from_leaf(index), Some(*expected));
        assert_eq!(Path::leaf(*expected), index);
    }
    assert_eq!(
        <RootPath as Path>::from_leaf(leaf_index(<Root as Fields>::COUNT)),
        None
    );
    assert_eq!(<RootPath as Path>::from_leaf(u16::MAX), None);
}

#[test]
fn from_leaf_is_const() {
    const FOURTH: Option<RootPath> = RootPath::from_leaf(3);
    const PAST_END: Option<RootPath> = RootPath::from_leaf(5);
    assert_eq!(FOURTH, Some(RootPath::Mid(MidPath::M)));
    assert_eq!(PAST_END, None);
}

#[test]
fn child_paths_are_relative() {
    assert_eq!(Path::leaf(LeafPath::Y), 1);
    assert_eq!(Path::leaf(MidPath::Child(LeafPath::Y)), 1);
    assert_eq!(Path::leaf(MidPath::M), 2);
    assert_eq!(<MidPath as Path>::from_leaf(2), Some(MidPath::M));
    assert_eq!(<MidPath as Path>::from_leaf(3), None);
    assert_eq!(<LeafPath as Path>::from_leaf(2), None);
}

#[test]
fn deep_write_drains_nested_path() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mid().child().x().set(1);
    assert_eq!(
        drained(&mut changes),
        vec![RootPath::Mid(MidPath::Child(LeafPath::X))]
    );
    assert_eq!(model.mid.child.x, 1);
    assert_eq!(drained(&mut changes), vec![]);
}

#[test]
fn writes_across_levels_drain_in_declaration_order() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.z().set(9);
        handle.mid().m().set(3);
        handle.mid().child().y().set(2);
        handle.a().set(1);
    }
    assert_eq!(
        drained(&mut changes),
        vec![
            RootPath::A,
            RootPath::Mid(MidPath::Child(LeafPath::Y)),
            RootPath::Mid(MidPath::M),
            RootPath::Z,
        ]
    );
    assert_eq!(
        model,
        Root {
            a: 1,
            mid: Mid {
                child: Leaf { x: 0, y: 2 },
                m: 3,
            },
            z: 9,
            meta: 0,
        }
    );
}

#[test]
fn projection() {
    assert_eq!(RootPath::Mid(MidPath::M).mid(), Some(MidPath::M));
    assert_eq!(RootPath::A.mid(), None);
    assert_eq!(RootPath::Z.mid(), None);
    assert_eq!(
        RootPath::Mid(MidPath::Child(LeafPath::X))
            .mid()
            .and_then(MidPath::child),
        Some(LeafPath::X)
    );
    assert_eq!(MidPath::Child(LeafPath::X).child(), Some(LeafPath::X));
    assert_eq!(MidPath::M.child(), None);
    assert_eq!(Path::leaf(MidPath::M), 2);
}

#[test]
fn subtree_set() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mid().set(Mid {
        child: Leaf { x: 4, y: 5 },
        m: 6,
    });
    assert_eq!(
        drained(&mut changes),
        vec![
            RootPath::Mid(MidPath::Child(LeafPath::X)),
            RootPath::Mid(MidPath::Child(LeafPath::Y)),
            RootPath::Mid(MidPath::M),
        ]
    );
    assert_eq!(
        model.mid,
        Mid {
            child: Leaf { x: 4, y: 5 },
            m: 6,
        }
    );
    assert_eq!(model.a, 0);
    assert_eq!(model.z, 0);
}

#[test]
fn subtree_mark_all() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mid().mark_all();
    assert_eq!(
        drained(&mut changes),
        vec![
            RootPath::Mid(MidPath::Child(LeafPath::X)),
            RootPath::Mid(MidPath::Child(LeafPath::Y)),
            RootPath::Mid(MidPath::M),
        ]
    );
    assert_eq!(model, root());
}

#[test]
fn root_set_and_mark_all_cover_every_leaf() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mark_all();
    assert_eq!(drained(&mut changes), ROOT_LEAVES.to_vec());
    changes.track(&mut model).set(Root {
        a: 1,
        mid: Mid {
            child: Leaf { x: 2, y: 3 },
            m: 4,
        },
        z: 5,
        meta: 6,
    });
    assert_eq!(drained(&mut changes), ROOT_LEAVES.to_vec());
    assert_eq!(model.meta, 6);
}

#[test]
fn model_reads_through_the_handle() {
    let mut model = root();
    let mut changes = Changes::<Root>::new();
    let mut handle = changes.track(&mut model);
    handle.mid().m().set(7);
    assert_eq!(handle.model().mid.m, 7);
    assert_eq!(handle.mid().model().m, 7);
    assert_eq!(handle.mid().child().model().x, 0);
}

#[test]
fn skipped_field_is_invisible() {
    assert_eq!(<Root as Fields>::COUNT, 5);
    assert_eq!(core::mem::size_of::<<Root as Fields>::Words>(), 4);
    let mut model = root();
    model.meta = 42;
    let mut changes = Changes::<Root>::new();
    changes.track(&mut model).mark_all();
    assert_eq!(changes.drain().count(), 5);
    assert_eq!(model.meta, 42);
}

#[test]
fn path_qualified_and_aliased_children() {
    assert_eq!(<Aliased as Fields>::COUNT, 3);
    let mut model = Aliased {
        mid: Mid {
            child: Leaf { x: 0, y: 0 },
            m: 0,
        },
        meta: 0,
    };
    let mut changes = Changes::<Aliased>::new();
    changes.track(&mut model).mid().child().y().set(1);
    changes.track(&mut model).mid().m().set(2);
    assert_eq!(
        drained(&mut changes),
        vec![
            AliasedPath::Mid(MidPath::Child(LeafPath::Y)),
            AliasedPath::Mid(MidPath::M),
        ]
    );
    assert_eq!(Path::leaf(AliasedPath::Mid(MidPath::M)), 2);
    assert_eq!(AliasedPath::Mid(MidPath::M).mid(), Some(MidPath::M));
    assert_eq!(model.mid.child.y, 1);
    assert_eq!(model.mid.m, 2);
}

#[test]
fn same_child_under_two_roots() {
    assert_eq!(<Other as Fields>::COUNT, 6);
    let expected = [
        OtherPath::Pad,
        OtherPath::One(LeafPath::X),
        OtherPath::One(LeafPath::Y),
        OtherPath::Mid(MidPath::Child(LeafPath::X)),
        OtherPath::Mid(MidPath::Child(LeafPath::Y)),
        OtherPath::Mid(MidPath::M),
    ];
    for (index, path) in expected.iter().enumerate() {
        assert_eq!(Path::leaf(*path), leaf_index(index));
        assert_eq!(
            <OtherPath as Path>::from_leaf(leaf_index(index)),
            Some(*path)
        );
    }
    assert_eq!(<OtherPath as Path>::from_leaf(6), None);

    let mut other = Other {
        pad: 0,
        one: Leaf { x: 0, y: 0 },
        mid: Mid {
            child: Leaf { x: 0, y: 0 },
            m: 0,
        },
    };
    let mut other_changes = Changes::<Other>::new();
    {
        let mut handle = other_changes.track(&mut other);
        handle.mid().child().y().set(1);
        handle.one().y().set(2);
        handle.pad().set(3);
    }
    assert_eq!(
        drained(&mut other_changes),
        vec![
            OtherPath::Pad,
            OtherPath::One(LeafPath::Y),
            OtherPath::Mid(MidPath::Child(LeafPath::Y)),
        ]
    );
    assert_eq!(other.pad, 3);
    assert_eq!(other.one.y, 2);
    assert_eq!(other.mid.child.y, 1);
    assert_eq!(OtherPath::One(LeafPath::Y).one(), Some(LeafPath::Y));
    assert_eq!(OtherPath::Pad.one(), None);
    assert_eq!(OtherPath::Mid(MidPath::M).mid(), Some(MidPath::M));

    let mut root_model = root();
    let mut root_changes = Changes::<Root>::new();
    root_changes.track(&mut root_model).mid().child().y().set(3);
    assert_eq!(
        drained(&mut root_changes),
        vec![RootPath::Mid(MidPath::Child(LeafPath::Y))]
    );
    assert_eq!(root_model.mid.child.y, 3);

    let mut leaf_model = Leaf { x: 0, y: 0 };
    let mut leaf_changes = Changes::<Leaf>::new();
    leaf_changes.track(&mut leaf_model).y().set(4);
    assert_eq!(drained(&mut leaf_changes), vec![LeafPath::Y]);
}

#[test]
fn path_sizes() {
    assert_eq!(core::mem::size_of::<LeafPath>(), 1);
    assert!(core::mem::size_of::<MidPath>() <= 2);
    assert!(core::mem::size_of::<RootPath>() <= 3);
    assert!(core::mem::size_of::<Option<RootPath>>() <= 4);
}

#[test]
fn leaf_fields_may_be_named_like_path_methods() {
    assert_eq!(<PathNamed as Fields>::COUNT, 2);
    assert_eq!(Path::leaf(PathNamedPath::Leaf), 0);
    assert_eq!(Path::leaf(PathNamedPath::FromLeaf), 1);
    assert_eq!(PathNamedPath::from_leaf(1), Some(PathNamedPath::FromLeaf));
    let mut model = PathNamed {
        leaf: 0,
        from_leaf: 0,
    };
    let mut changes = Changes::<PathNamed>::new();
    changes.track(&mut model).from_leaf().set(1);
    assert_eq!(drained(&mut changes), vec![PathNamedPath::FromLeaf]);
    assert_eq!(model.from_leaf, 1);
    assert_eq!(model.leaf, 0);
}
