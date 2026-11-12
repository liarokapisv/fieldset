use fieldset::Fields;

#[derive(Fields)]
struct S {
    a: u8,
}

#[derive(Fields)]
struct X {
    #[fields(nested, nested)]
    s: S,
}

fn main() {}
