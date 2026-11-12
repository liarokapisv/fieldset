use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields()]
    a: u8,
}

fn main() {}
