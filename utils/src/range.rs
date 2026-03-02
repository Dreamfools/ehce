use num_traits::PrimInt;
use std::collections::Bound;
use std::ops::{Range, RangeBounds};

pub fn clamp_bounds<T: Copy + PrimInt, R: RangeBounds<T>>(bound: R, clamp: Range<T>) -> Range<T> {
    let min = match bound.start_bound() {
        Bound::Included(v) => *v,
        Bound::Excluded(v) => *v + T::one(),
        Bound::Unbounded => clamp.start,
    }
    .clamp(clamp.start, clamp.end);

    let max = match bound.end_bound() {
        Bound::Included(v) => *v + T::one(),
        Bound::Excluded(v) => *v,
        Bound::Unbounded => clamp.end,
    }
    .clamp(clamp.start, clamp.end);

    min..max
}
