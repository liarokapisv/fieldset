use fieldset::Changes;

mod a {
    use fieldset::Fields;

    #[derive(Fields)]
    pub struct Inner {
        pub x: u8,
    }
}

mod b {
    use fieldset::Fields;

    #[derive(Fields)]
    pub struct Outer {
        #[fields(nested)]
        pub inner: super::a::Inner,
    }
}

fn main() {
    let mut model = b::Outer {
        inner: a::Inner { x: 0 },
    };
    let mut changes = Changes::<b::Outer>::new();
    changes.track(&mut model).inner().x().set(1);
    let mut drain = changes.drain();
    assert_eq!(drain.next(), Some(b::OuterPath::Inner(a::InnerPath::X)));
    assert_eq!(drain.next(), None);
    assert_eq!(model.inner.x, 1);
}
