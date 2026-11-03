//! Private best-fit extent allocation; live ranges never move.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct Extent {
    pub start: usize,
    pub len: usize,
}

#[derive(Debug, Default)]
pub(super) struct Extents {
    pub end: usize,
    pub peak: usize,
    pub allocated: usize,
    offsets: BTreeMap<usize, usize>,
    sizes: BTreeSet<(usize, usize)>,
}
impl Extents {
    pub fn reserve(&mut self, len: usize, capacity: usize) -> Option<Extent> {
        if len == 0 {
            return Some(Extent { start: 0, len });
        }
        let hole = self.sizes.range((len, 0)..).next().copied();
        let start = if let Some((size, start)) = hole {
            self.remove(start, size);
            if size > len {
                self.insert(start + len, size - len);
            }
            start
        } else {
            if len > capacity - self.end {
                return None;
            }
            let start = self.end;
            self.end += len;
            self.peak = self.peak.max(self.end);
            start
        };
        self.allocated += len;
        Some(Extent { start, len })
    }
    /// Releases a range and reports whether the reserved tail shrank.
    pub fn release(&mut self, extent: Extent) -> bool {
        if extent.len == 0 {
            return false;
        }
        self.allocated -= extent.len;
        let mut start = extent.start;
        let mut len = extent.len;
        if let Some((&before, &size)) = self.offsets.range(..start).next_back()
            && before + size == start
        {
            self.remove(before, size);
            start = before;
            len += size;
        }
        if let Some(&size) = self.offsets.get(&(start + len)) {
            self.remove(start + len, size);
            len += size;
        }
        if start + len == self.end {
            self.end = start;
            true
        } else {
            self.insert(start, len);
            false
        }
    }
    fn insert(&mut self, start: usize, len: usize) {
        self.offsets.insert(start, len);
        self.sizes.insert((len, start));
    }
    fn remove(&mut self, start: usize, len: usize) {
        self.offsets.remove(&start);
        self.sizes.remove(&(len, start));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_fit_preserves_larger_gaps_and_split_remainders() {
        let mut allocator = Extents::default();
        let a = allocator.reserve(10, 100).unwrap();
        let b = allocator.reserve(20, 100).unwrap();
        let c = allocator.reserve(30, 100).unwrap();
        let d = allocator.reserve(40, 100).unwrap();
        allocator.release(a);
        allocator.release(c);
        let small = allocator.reserve(8, 100).unwrap();
        assert_eq!(small.start, 0);
        let large = allocator.reserve(30, 100).unwrap();
        assert_eq!(large.start, 30);
        let remainder = allocator.reserve(2, 100).unwrap();
        assert_eq!(remainder.start, 8);
        assert!(allocator.reserve(1, 100).is_none());
        for extent in [small, large, b, remainder, d] {
            allocator.release(extent);
        }
        assert_eq!(allocator.end, 0);
        assert_eq!(allocator.allocated, 0);
        assert_eq!(allocator.peak, 100);
        assert!(allocator.offsets.is_empty());
        assert!(allocator.sizes.is_empty());
    }

    #[test]
    fn mixed_allocations_match_an_independent_byte_occupancy_model() {
        for seed in 1..=8u64 {
            let mut random = seed;
            let mut allocator = Extents::default();
            let mut active = Vec::<Extent>::new();
            let mut occupied = [false; 127];
            let mut peak = 0;
            for _ in 0..2000 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let end = occupied.iter().rposition(|used| *used).map_or(0, |i| i + 1);
                if random % 3 == 0 && !active.is_empty() {
                    let extent = active.swap_remove((random as usize / 3) % active.len());
                    for used in &mut occupied[extent.start..extent.start + extent.len] {
                        assert!(*used);
                        *used = false;
                    }
                    let next_end = occupied.iter().rposition(|used| *used).map_or(0, |i| i + 1);
                    assert_eq!(allocator.release(extent), next_end < end);
                } else {
                    let len = (random as usize / 7) % 24;
                    let mut holes = Vec::new();
                    let mut at = 0;
                    while at < end {
                        if occupied[at] {
                            at += 1;
                            continue;
                        }
                        let start = at;
                        while at < end && !occupied[at] {
                            at += 1;
                        }
                        holes.push((at - start, start));
                    }
                    holes.sort();
                    let expected = if len == 0 {
                        Some(0)
                    } else {
                        holes
                            .iter()
                            .find(|(size, _)| *size >= len)
                            .map(|(_, start)| *start)
                            .or_else(|| (end + len <= occupied.len()).then_some(end))
                    };
                    let extent = allocator.reserve(len, occupied.len());
                    assert_eq!(extent.as_ref().map(|e| e.start), expected);
                    if let Some(extent) = extent {
                        for used in &mut occupied[extent.start..extent.start + len] {
                            assert!(!*used);
                            *used = true;
                        }
                        active.push(extent);
                    }
                }
                let end = occupied.iter().rposition(|used| *used).map_or(0, |i| i + 1);
                peak = peak.max(end);
                assert_eq!(allocator.end, end);
                assert_eq!(
                    allocator.allocated,
                    occupied.iter().filter(|used| **used).count()
                );
                assert_eq!(allocator.peak, peak);
            }
            for extent in active {
                allocator.release(extent);
            }
            assert_eq!(allocator.end, 0);
            assert!(allocator.offsets.is_empty());
            assert!(allocator.sizes.is_empty());
        }
    }
}
