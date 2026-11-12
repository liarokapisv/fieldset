use fieldset::Fields;

#[derive(Fields)]
struct Two {
    a: u8,
    b: f32,
}

static STATIC_CHANGES: fieldset::Changes<Two> = fieldset::Changes::new();

#[test]
fn two_writes_drain_in_declaration_order() {
    let mut model = Two { a: 0, b: 0.0 };
    let mut changes = fieldset::Changes::<Two>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.b().set(1.5);
        handle.a().set(7);
    }
    let mut drain = changes.drain();
    assert_eq!(drain.next(), Some(TwoPath::A));
    assert_eq!(drain.next(), Some(TwoPath::B));
    assert_eq!(drain.next(), None);
    assert_eq!(model.a, 7);
    assert_eq!(model.b, 1.5);
}

#[test]
fn same_leaf_twice_yields_once() {
    let mut model = Two { a: 0, b: 0.0 };
    let mut changes = fieldset::Changes::<Two>::new();
    {
        let mut handle = changes.track(&mut model);
        handle.a().set(1);
        handle.a().set(2);
    }
    let mut drain = changes.drain();
    assert_eq!(drain.next(), Some(TwoPath::A));
    assert_eq!(drain.next(), None);
    assert_eq!(model.a, 2);
}

#[test]
fn empty_drain_yields_nothing() {
    let mut changes = fieldset::Changes::<Two>::new();
    assert_eq!(changes.drain().next(), None);
}

#[test]
fn drain_clears() {
    let mut model = Two { a: 0, b: 0.0 };
    let mut changes = fieldset::Changes::<Two>::new();
    changes.track(&mut model).a().set(3);
    changes.track(&mut model).b().set(4.0);
    assert_eq!(changes.drain().count(), 2);
    assert_eq!(changes.drain().next(), None);
}

#[test]
fn leaf_round_trip() {
    let count = u16::try_from(<Two as Fields>::COUNT).ok();
    assert_eq!(count, Some(2));
    for leaf in 0..<Two as Fields>::COUNT {
        let leaf = u16::try_from(leaf).ok();
        assert_eq!(
            leaf.and_then(<TwoPath as fieldset::Path>::from_leaf)
                .map(fieldset::Path::leaf),
            leaf
        );
    }
    assert_eq!(count.and_then(<TwoPath as fieldset::Path>::from_leaf), None);
}

#[test]
fn changes_lives_in_a_static() {
    assert_eq!(
        core::mem::size_of_val(&STATIC_CHANGES),
        core::mem::size_of::<<Two as Fields>::Words>()
    );
}
