#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![no_std]
#![forbid(unsafe_code)]

mod changes;
mod field;
mod fields;
mod index;
mod mask;

pub use changes::{Changes, Drain, Iter};
pub use field::Field;
pub use fields::{Fields, Path, Words};
pub use index::Index;
pub use mask::{Mask, MaskOps};

pub use fieldset_macro::Fields;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_docs_are_the_readme() {
        const README: &str = include_str!("../README.md");
        assert!(README.starts_with("# fieldset"));
    }
}
