/// A bounds-checked element index into an indexed field of `N` elements.
///
/// Carried by the path variants of indexed fields and taken by their handle
/// methods; the only way to obtain one is [`Index::new`], so an index that
/// exists is always in range.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Index<const N: usize>(u16);

impl<const N: usize> Index<N> {
    const FITS: () = assert!(
        N <= u16::MAX as usize + 1,
        "an indexed field holds at most 65536 elements"
    );

    /// The index `i`, or `None` when `i >= N`.
    pub const fn new(i: usize) -> Option<Self> {
        let () = Self::FITS;
        if i < N {
            Some(Self(i as u16))
        } else {
            None
        }
    }

    /// The element position this index names, always below `N`.
    pub const fn get(self) -> usize {
        self.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::Index;

    #[test]
    fn in_range_round_trips() {
        for i in 0..4 {
            assert_eq!(Index::<4>::new(i).map(Index::get), Some(i));
        }
    }

    #[test]
    fn out_of_range_is_none() {
        assert_eq!(Index::<4>::new(4), None);
        assert_eq!(Index::<4>::new(usize::MAX), None);
        assert_eq!(Index::<0>::new(0), None);
    }

    #[test]
    fn largest_bound_fits() {
        const LAST: Option<Index<65536>> = Index::new(65535);
        assert_eq!(LAST.map(Index::get), Some(65535));
        assert_eq!(Index::<65536>::new(65536), None);
    }

    #[test]
    fn is_one_u16_wide() {
        assert_eq!(core::mem::size_of::<Index<4>>(), 2);
    }
}
