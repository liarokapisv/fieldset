use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(indexed)]
    a: u8,
}

fn main() {}
