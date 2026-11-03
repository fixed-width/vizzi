//! Aggregate admission for two stable backing-file indexes.
use super::extents::{Extent, Extents};

#[derive(Debug)]
pub(super) struct Allocation {
    pub bank: usize,
    pub extent: Extent,
}
#[derive(Debug, Default)]
pub(super) struct Bank {
    pub extents: Extents,
    pub charged: usize,
    pub cleaning: bool,
}
#[derive(Debug, Default)]
pub(super) struct Pool {
    pub banks: [Bank; 2],
    pub active: usize,
    pub peak: usize,
}
#[derive(Debug)]
pub(super) struct Cut {
    pub bank: usize,
    pub end: usize,
}
impl Pool {
    pub fn charged(&self) -> usize {
        self.banks.iter().map(|b| b.charged).sum()
    }
    pub fn allocated(&self) -> usize {
        self.banks.iter().map(|b| b.extents.allocated).sum()
    }
    pub fn reserve(&mut self, len: usize, capacity: usize) -> Option<Allocation> {
        let total = self.charged();
        for bank in [self.active, 1 - self.active] {
            let b = &mut self.banks[bank];
            if b.cleaning {
                continue;
            }
            let limit = b.charged + (capacity - total);
            if let Some(extent) = b.extents.reserve(len, limit) {
                b.charged = b.charged.max(b.extents.end);
                self.active = bank;
                self.peak = self.peak.max(self.charged());
                return Some(Allocation { bank, extent });
            }
        }
        None
    }
    pub fn retire(&mut self, allocation: Allocation) {
        self.banks[allocation.bank]
            .extents
            .release(allocation.extent);
    }
    pub fn begin_clean(&mut self, bank: usize) -> Option<Cut> {
        let b = &mut self.banks[bank];
        if b.cleaning || b.charged == b.extents.end {
            return None;
        }
        b.cleaning = true;
        if self.active == bank {
            self.active = 1 - bank;
        }
        Some(Cut {
            bank,
            end: b.extents.end,
        })
    }
    pub fn finish_clean(&mut self, cut: Cut, success: bool) {
        let b = &mut self.banks[cut.bank];
        if success {
            b.charged = cut.end;
        }
        b.cleaning = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleaning_charge_stays_reserved_while_the_other_file_progresses() {
        let mut pool = Pool::default();
        let pin = pool.reserve(8, 24).unwrap();
        let tail = pool.reserve(8, 24).unwrap();
        pool.retire(tail);
        let cut = pool.begin_clean(0).unwrap();
        assert_eq!(pool.charged(), 16);
        let next = pool.reserve(8, 24).unwrap();
        assert_eq!(next.bank, 1);
        assert_eq!(pool.charged(), 24);
        assert!(pool.reserve(1, 24).is_none());
        pool.finish_clean(cut, true);
        assert_eq!(pool.charged(), 16);
        pool.retire(pin);
        pool.retire(next);
        for bank in 0..2 {
            let cut = pool.begin_clean(bank).unwrap();
            pool.finish_clean(cut, true);
        }
        assert_eq!(pool.charged(), 0);
        assert_eq!(pool.allocated(), 0);
        assert_eq!(pool.peak, 24);
    }
    #[test]
    fn failure_and_later_retirement_do_not_release_unfinished_physical_charge() {
        let mut pool = Pool::default();
        let pin = pool.reserve(4, 12).unwrap();
        let tail = pool.reserve(8, 12).unwrap();
        pool.retire(tail);
        let cut = pool.begin_clean(0).unwrap();
        assert!(pool.begin_clean(0).is_none());
        assert!(pool.reserve(1, 12).is_none());
        pool.retire(pin);
        pool.finish_clean(cut, false);
        assert_eq!(pool.charged(), 12);
        let retry = pool.begin_clean(0).unwrap();
        assert_eq!(retry.end, 0);
        pool.finish_clean(retry, true);
        let all = pool.reserve(12, 12).unwrap();
        pool.retire(all);
        assert_eq!(pool.charged(), 12);
        assert!(pool.begin_clean(0).is_none());
        let cut = pool.begin_clean(1).unwrap();
        pool.finish_clean(cut, true);
    }
    #[test]
    fn retired_tail_can_be_overwritten_without_uncharging_or_truncating() {
        let mut pool = Pool::default();
        let old = pool.reserve(12, 12).unwrap();
        pool.retire(old);
        let next = pool.reserve(5, 12).unwrap();
        assert_eq!(next.bank, 0);
        assert_eq!(next.extent.start, 0);
        assert_eq!(pool.charged(), 12);
        let cut = pool.begin_clean(0).unwrap();
        assert_eq!(cut.end, 5);
        pool.finish_clean(cut, true);
        assert_eq!(pool.charged(), 5);
        assert_eq!(pool.allocated(), 5);
    }
    #[test]
    fn mixed_file_admission_matches_independent_occupancy_and_physical_charges() {
        for seed in 1..=8u64 {
            let mut random = seed;
            let mut pool = Pool::default();
            let mut occupied = [[false; 127]; 2];
            let mut charged = [0usize; 2];
            let mut cuts: [Option<Cut>; 2] = [None, None];
            let mut allocations = Vec::<Allocation>::new();
            let mut peak = 0;
            for _ in 0..2000 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let bank = (random as usize / 11) % 2;
                match (random >> 32) % 4 {
                    0 => {
                        let len = (random as usize / 17) % 24;
                        let total: usize = charged.iter().sum();
                        let available = (0..2).any(|i| {
                            cuts[i].is_none() && {
                                let limit = charged[i] + 127 - total;
                                if len == 0 {
                                    true
                                } else {
                                    (0..=limit.saturating_sub(len)).any(|start| {
                                        start + len <= limit
                                            && occupied[i][start..start + len].iter().all(|b| !*b)
                                    })
                                }
                            }
                        });
                        let result = pool.reserve(len, 127);
                        assert_eq!(result.is_some(), available);
                        if let Some(a) = result {
                            assert!(cuts[a.bank].is_none());
                            for used in &mut occupied[a.bank][a.extent.start..a.extent.start + len]
                            {
                                assert!(!*used);
                                *used = true;
                            }
                            charged[a.bank] = charged[a.bank].max(a.extent.start + len);
                            allocations.push(a);
                        }
                    }
                    1 if !allocations.is_empty() => {
                        let a = allocations.swap_remove((random as usize / 19) % allocations.len());
                        for used in
                            &mut occupied[a.bank][a.extent.start..a.extent.start + a.extent.len]
                        {
                            assert!(*used);
                            *used = false;
                        }
                        pool.retire(a);
                    }
                    2 => {
                        let end = occupied[bank].iter().rposition(|b| *b).map_or(0, |i| i + 1);
                        let expected = cuts[bank].is_none() && charged[bank] > end;
                        let cut = pool.begin_clean(bank);
                        assert_eq!(cut.is_some(), expected);
                        if let Some(cut) = cut {
                            assert_eq!(cut.end, end);
                            cuts[bank] = Some(cut);
                        }
                    }
                    _ => {
                        if let Some(cut) = cuts[bank].take() {
                            let success = random % 7 != 0;
                            if success {
                                charged[bank] = cut.end;
                            }
                            pool.finish_clean(cut, success);
                        }
                    }
                }
                let total: usize = charged.iter().sum();
                peak = peak.max(total);
                assert!(total <= 127);
                assert_eq!(pool.charged(), total);
                assert_eq!(pool.peak, peak);
                assert_eq!(
                    pool.allocated(),
                    occupied.iter().flatten().filter(|b| **b).count()
                );
                for bank in 0..2 {
                    assert_eq!(pool.banks[bank].charged, charged[bank]);
                    assert_eq!(pool.banks[bank].cleaning, cuts[bank].is_some());
                    assert_eq!(
                        pool.banks[bank].extents.end,
                        occupied[bank].iter().rposition(|b| *b).map_or(0, |i| i + 1)
                    );
                }
            }
        }
    }
}
