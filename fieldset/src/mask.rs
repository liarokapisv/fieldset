/// A leaf bitset viewed as a word slice, with the root type erased.
pub type Mask = [u32];

mod sealed {
    pub trait Sealed {}

    impl Sealed for [u32] {}
}

/// Bit operations on a [`Mask`].
///
/// Leaf indices are bit positions; a range is half-open. The trait is sealed:
/// it is implemented for [`Mask`] only, and generated code calls it on behalf
/// of a [`Changes`](crate::Changes) set.
pub trait MaskOps: sealed::Sealed {
    /// Sets the bit for one leaf.
    fn set_bit(&mut self, leaf: u16);

    /// Tells whether the bit for one leaf is set.
    fn test_bit(&self, leaf: u16) -> bool;

    /// Sets every bit in `start..end`.
    fn set_range(&mut self, start: u16, end: u16);

    /// Clears every bit in `start..end`.
    fn clear_range(&mut self, start: u16, end: u16);
}

impl MaskOps for Mask {
    fn set_bit(&mut self, leaf: u16) {
        let leaf = usize::from(leaf);
        self[leaf / 32] |= 1 << (leaf % 32);
    }

    fn test_bit(&self, leaf: u16) -> bool {
        let leaf = usize::from(leaf);
        self[leaf / 32] & (1 << (leaf % 32)) != 0
    }

    fn set_range(&mut self, start: u16, end: u16) {
        for_each_word(self, start, end, |word, bits| *word |= bits);
    }

    fn clear_range(&mut self, start: u16, end: u16) {
        for_each_word(self, start, end, |word, bits| *word &= !bits);
    }
}

fn for_each_word(words: &mut Mask, start: u16, end: u16, mut apply: impl FnMut(&mut u32, u32)) {
    let start = usize::from(start);
    let end = usize::from(end);
    if start >= end {
        return;
    }
    let first = start / 32;
    let last = (end - 1) / 32;
    for (offset, word) in words[first..=last].iter_mut().enumerate() {
        let base = (first + offset) * 32;
        let low = start.max(base) - base;
        let high = end.min(base + 32) - base;
        apply(word, (u32::MAX >> (32 - (high - low))) << low);
    }
}
