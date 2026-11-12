use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(indexed)]
    g: [[u8; 2]; 3],
}

fn main() {}
