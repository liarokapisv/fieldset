use fieldset::Fields;

#[derive(Fields)]
struct X<T: Copy> {
    a: T,
}

fn main() {}
