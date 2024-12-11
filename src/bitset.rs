#[derive(Clone, Copy, Debug)]
pub struct BitSet<const N_32: usize> {
    bits: [u32; N_32],
}

pub struct BitSetOffsetted<'a> {
    bits: &'a mut [u32],
    offset: usize,
}

impl<'a> BitSetOffsetted<'a> {
    pub fn test(&self, i: usize) -> bool {
        self.bits[self.idx(i) / 32] & (1 << (self.idx(i) % 32)) != 0
    }

    pub fn set(&mut self, i: usize) {
        self.bits[self.idx(i) / 32] |= 1 << (self.idx(i) % 32);
    }

    pub fn clear(&mut self, i: usize) {
        self.bits[self.idx(i) / 32] &= !(1 << (self.idx(i) % 32));
    }

    pub fn offset(&mut self, offset: usize) -> BitSetOffsetted {
        BitSetOffsetted {
            bits: &mut self.bits,
            offset: self.offset + offset,
        }
    }

    fn idx(&self, index: usize) -> usize {
        self.offset + index
    }
}

impl<const N_32: usize> BitSet<N_32> {
    pub fn new() -> Self {
        Self { bits: [0; N_32] }
    }

    pub fn test(&self, i: usize) -> bool {
        self.bits[i / 32] & (1 << (i % 32)) != 0
    }

    pub fn set(&mut self, i: usize, x: bool) {
        if x {
            self.bits[i / 32] |= 1 << (i % 32);
        } else {
            self.bits[i / 32] &= !(1 << (i % 32));
        }
    }

    pub fn offset(&mut self, offset: usize) -> BitSetOffsetted {
        BitSetOffsetted {
            bits: &mut self.bits,
            offset,
        }
    }
}
