use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(nested)]
    s: u8,
}

fn main() {}
