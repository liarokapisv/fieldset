use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(bogus)]
    a: u8,
}

fn main() {}
