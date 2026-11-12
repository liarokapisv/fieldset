use fieldset::Fields;

#[derive(Fields)]
struct Inner {
    x: u8,
}

#[derive(Fields)]
struct Outer {
    a: u8,
    #[fields(nested)]
    inner: Inner,
}

fn main() {
    let mut model = Outer {
        a: 0,
        inner: Inner { x: 0 },
    };
    let mut changes = fieldset::Changes::<Outer>::new();
    changes.track(&mut model).inner().x().set(2);
    let mut drain = changes.drain();
    let path = drain.next();
    assert_eq!(path, Some(OuterPath::Inner(InnerPath::X)));
    assert_eq!(path.map(fieldset::Path::leaf), Some(1));
    assert_eq!(drain.next(), None);
    assert_eq!(model.a, 0);
    assert_eq!(model.inner.x, 2);
}
