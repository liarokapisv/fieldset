use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(skip)]
    a: u8,
}

fn main() {}
