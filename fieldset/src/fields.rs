use crate::mask::Mask;

/// A struct whose leaves can be addressed, written through a handle and
/// tracked in a [`Changes`](crate::Changes) set.
///
/// Implemented by `#[derive(Fields)]`; never by hand.
pub trait Fields: Sized {
    /// Number of leaves in this struct.
    const COUNT: usize;

    /// The word array that holds one bit per leaf: exactly
    /// `COUNT.div_ceil(32)` words, as the derive spells it.
    type Words: Words;

    /// The path enum that names one leaf of this struct.
    type Path: Path;

    /// The write handle over one borrow of this struct.
    type Mut<'a>
    where
        Self: 'a;

    #[doc(hidden)]
    fn handle<'a>(&'a mut self, mask: &'a mut Mask, base: u16) -> Self::Mut<'a>;
}

/// A reference to one leaf of a struct, as data.
pub trait Path: Copy + Eq + core::fmt::Debug {
    /// The leaf index of this path, relative to its own struct.
    fn leaf(self) -> u16;

    /// The path for a leaf index, or `None` when the index is out of range.
    fn from_leaf(leaf: u16) -> Option<Self>;
}

/// Fixed-size word storage for a leaf bitset.
pub trait Words: Copy {
    /// The storage with every bit clear.
    const ZERO: Self;

    /// Views the storage as a word slice.
    fn as_mask(&self) -> &Mask;

    /// Views the storage as a mutable word slice.
    fn as_mask_mut(&mut self) -> &mut Mask;
}

impl<const N: usize> Words for [u32; N] {
    const ZERO: Self = [0; N];

    fn as_mask(&self) -> &Mask {
        self
    }

    fn as_mask_mut(&mut self) -> &mut Mask {
        self
    }
}
