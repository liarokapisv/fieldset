use fieldset::Fields;

#[derive(Fields)]
struct X<'a> {
    a: &'a u8,
}

fn main() {}
