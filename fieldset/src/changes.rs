use core::marker::PhantomData;

use crate::fields::{Fields, Path, Words};
use crate::mask::{Mask, MaskOps};

/// The set of leaves of `R` written since the last drain.
///
/// One bit per leaf, nothing else; plain data that lives wherever the host
/// keeps it, a `static` included. It holds only the word array, so it is
/// `Send` and `Sync` whatever `R` is.
pub struct Changes<R: Fields> {
    words: R::Words,
    _r: PhantomData<fn() -> R>,
}

impl<R: Fields> Changes<R> {
    /// An empty set.
    #[allow(
        clippy::new_without_default,
        reason = "the public surface is fixed; `new` is the only constructor"
    )]
    pub const fn new() -> Self {
        Self {
            words: <R::Words as Words>::ZERO,
            _r: PhantomData,
        }
    }

    /// Borrows `model` through a handle whose writes are recorded here.
    pub fn track<'a>(&'a mut self, model: &'a mut R) -> R::Mut<'a> {
        <R as Fields>::handle(model, self.words.as_mask_mut(), 0)
    }

    /// Yields every marked leaf in ascending order, clearing each as it goes.
    ///
    /// Dropping the iterator early leaves the unvisited leaves marked, so a
    /// later `drain` or [`iter`](Self::iter) resumes with them.
    pub fn drain(&mut self) -> Drain<'_, R> {
        Drain {
            words: self.words.as_mask_mut(),
            index: 0,
            _r: PhantomData,
        }
    }

    /// Yields every marked leaf in ascending order, leaving all of them marked.
    ///
    /// The order is the one [`drain`](Self::drain) would use.
    pub fn iter(&self) -> Iter<'_, R> {
        Iter {
            words: self.words.as_mask(),
            index: 0,
            current: 0,
            _r: PhantomData,
        }
    }

    /// Marks every leaf of `R`, and no bit beyond them.
    pub fn mark_all(&mut self) {
        let mask = self.words.as_mask_mut();
        match u16::try_from(R::COUNT) {
            Ok(end) => mask.set_range(0, end),
            Err(_) => mask.fill(u32::MAX),
        }
    }

    /// Clears every leaf.
    pub fn clear(&mut self) {
        self.words.as_mask_mut().fill(0);
    }

    /// The number of marked leaves.
    pub fn len(&self) -> usize {
        self.words
            .as_mask()
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Tells whether no leaf is marked.
    pub fn is_empty(&self) -> bool {
        self.words.as_mask().iter().all(|word| *word == 0)
    }

    #[doc(hidden)]
    pub fn as_mask_mut(&mut self) -> &mut Mask {
        self.words.as_mask_mut()
    }
}

/// The iterator returned by [`Changes::drain`].
///
/// Each path it yields is cleared from the set as it is yielded; the paths
/// not yet visited when it is dropped stay marked.
pub struct Drain<'a, R: Fields> {
    words: &'a mut Mask,
    index: usize,
    _r: PhantomData<fn() -> R>,
}

impl<R: Fields> Iterator for Drain<'_, R> {
    type Item = R::Path;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let word = self.words.get_mut(self.index)?;
            if *word == 0 {
                self.index += 1;
                continue;
            }
            let bit = word.trailing_zeros();
            *word &= !(1 << bit);
            let leaf = self.index * 32 + bit as usize;
            if let Some(path) = u16::try_from(leaf).ok().and_then(R::Path::from_leaf) {
                return Some(path);
            }
        }
    }
}

/// The iterator returned by [`Changes::iter`].
///
/// Yields the marked paths in the order [`Changes::drain`] would, without
/// clearing any of them.
pub struct Iter<'a, R: Fields> {
    words: &'a Mask,
    index: usize,
    current: u32,
    _r: PhantomData<fn() -> R>,
}

impl<R: Fields> Iterator for Iter<'_, R> {
    type Item = R::Path;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            while self.current == 0 {
                self.current = *self.words.get(self.index)?;
                self.index += 1;
            }
            let bit = self.current.trailing_zeros();
            self.current &= self.current - 1;
            let leaf = (self.index - 1) * 32 + bit as usize;
            if let Some(path) = u16::try_from(leaf).ok().and_then(R::Path::from_leaf) {
                return Some(path);
            }
        }
    }
}
