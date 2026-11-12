use crate::mask::{Mask, MaskOps};

/// A handle over one leaf.
///
/// Returned by the leaf methods of a generated handle; every write through it
/// marks the leaf in the owning [`Changes`](crate::Changes) set, and a read
/// marks nothing.
pub struct Field<'a, T> {
    value: &'a mut T,
    mask: &'a mut Mask,
    leaf: u16,
}

impl<'a, T: Copy> Field<'a, T> {
    #[doc(hidden)]
    pub fn new(value: &'a mut T, mask: &'a mut Mask, leaf: u16) -> Self {
        Self { value, mask, leaf }
    }

    /// Reads the leaf without marking it.
    pub fn get(&self) -> T {
        *self.value
    }

    /// Writes the leaf and marks it, whatever the old value was.
    pub fn set(&mut self, value: T) {
        *self.value = value;
        self.mask.set_bit(self.leaf);
    }

    /// Writes the leaf and marks it only when the old value differs by
    /// [`PartialEq`].
    ///
    /// A leaf that is not equal to itself, such as a NaN float, is written
    /// and marked every time.
    pub fn set_if_changed(&mut self, value: T)
    where
        T: PartialEq,
    {
        if *self.value != value {
            self.set(value);
        }
    }

    /// Mutates the leaf in place and marks it, even when `f` leaves it as is.
    pub fn update(&mut self, f: impl FnOnce(&mut T)) {
        f(self.value);
        self.mask.set_bit(self.leaf);
    }
}
