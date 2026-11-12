#![deny(private_interfaces)]

mod model {
    use fieldset::Fields;

    #[derive(Fields)]
    struct Inner {
        x: u8,
    }

    #[derive(Fields)]
    pub struct Outer {
        #[fields(nested)]
        inner: Inner,
    }
}

fn main() {
    let path = model::OuterPath::Inner(model::InnerPath::X);
    let _ = path;
}
