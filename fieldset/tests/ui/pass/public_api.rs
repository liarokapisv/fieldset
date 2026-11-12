#![deny(private_interfaces)]

use fieldset::Changes;

pub mod api {
    use fieldset::Fields;

    #[derive(Fields)]
    pub struct Public {
        #[fields(nested)]
        pub inner: Inner,
    }

    #[derive(Fields)]
    pub struct Inner {
        pub x: u8,
    }
}

fn main() {
    let mut model = api::Public {
        inner: api::Inner { x: 0 },
    };
    let mut changes = Changes::<api::Public>::new();
    changes.track(&mut model).inner().x().set(1);
    let mut drain = changes.drain();
    assert_eq!(
        drain.next(),
        Some(api::PublicPath::Inner(api::InnerPath::X))
    );
    assert_eq!(drain.next(), None);
    assert_eq!(model.inner.x, 1);
}
