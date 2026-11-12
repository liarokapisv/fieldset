use fieldset::Fields;

#[derive(Fields)]
struct X {
    #[fields(indexed)]
    v: [u8; 65536],
}

fn main() {}
