use num_traits::{NumAssignRef, PrimInt};
use std::fmt::Debug;
use thiserror::Error;

/// Rect specified by numeric coordinates
///
/// left and top are inclusive, right and bottom are exclusive
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NumRect<T: PrimInt + NumAssignRef + Debug> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
}

#[derive(Debug, Error)]
#[error("Cannot shrink rectangle {:?} by {:?}: amount is larger than half of the rectangle's width or height", .rect, .amount)]
pub struct NumRectShrinkError<T: PrimInt + NumAssignRef + Debug> {
    pub rect: NumRect<T>,
    pub amount: T,
}

impl<T: PrimInt + NumAssignRef + Debug> NumRect<T> {
    pub fn new(left: T, top: T, right: T, bottom: T) -> Self {
        assert!(
            left <= right && top <= bottom,
            "Invalid rectangle coordinates"
        );
        NumRect {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> T {
        self.right - self.left
    }

    pub fn height(&self) -> T {
        self.bottom - self.top
    }

    /// Shrink the rectangle by the specified amount
    ///
    /// If the amount is zero or negative, no change is made
    ///
    /// # Panics
    /// If the amount is larger than half of the rectangle's width or height
    pub fn shrink_in_place(&mut self, amount: T) {
        self.try_shrink_in_place(amount).unwrap();
    }

    /// Try to shrink the rectangle by the specified amount
    ///
    /// If the amount is zero or negative, no change is made
    pub fn try_shrink_in_place(&mut self, amount: T) -> Result<(), NumRectShrinkError<T>> {
        if amount <= T::zero() {
            return Ok(());
        }

        let two = T::one() + T::one();
        if self.width() < amount * two || self.height() < amount * two {
            return Err(NumRectShrinkError {
                rect: *self,
                amount,
            });
        }

        self.left += amount;
        self.top += amount;
        self.right -= amount;
        self.bottom -= amount;
        Ok(())
    }

    pub fn try_shrink(&self, amount: T) -> Result<NumRect<T>, NumRectShrinkError<T>> {
        let mut new_rect = *self;
        new_rect.try_shrink_in_place(amount)?;
        Ok(new_rect)
    }

    /// Grow the rectangle by the specified amount
    ///
    /// If the amount is zero or negative, no change is made
    pub fn grow(&mut self, amount: T) {
        if amount <= T::zero() {
            return;
        }

        self.left -= amount;
        self.top -= amount;
        self.right += amount;
        self.bottom += amount;
    }

    pub fn border(&self) -> impl Iterator<Item = [T; 2]> {
        BorderIter::new(self)
    }
}

enum BorderIter<'a, T: PrimInt + NumAssignRef + Debug> {
    Top(&'a NumRect<T>, T),
    Right(&'a NumRect<T>, T),
    Bottom(&'a NumRect<T>, T),
    Left(&'a NumRect<T>, T),
    Done,
}

impl<'a, T: PrimInt + NumAssignRef + Debug> BorderIter<'a, T> {
    fn new(rect: &'a NumRect<T>) -> Self {
        if rect.width().is_zero() || rect.height().is_zero() {
            BorderIter::Done
        } else {
            BorderIter::Top(rect, rect.left)
        }
    }
}

impl<'a, T: PrimInt + NumAssignRef + Debug> Iterator for BorderIter<'a, T> {
    type Item = [T; 2];

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BorderIter::Top(rect, left) => {
                let coord = [*left, rect.top];
                *left += T::one();
                if *left >= rect.right {
                    if rect.height() == T::one() {
                        *self = BorderIter::Done;
                    } else {
                        *self = BorderIter::Right(rect, rect.top + T::one());
                    }
                }
                Some(coord)
            }
            BorderIter::Right(rect, top) => {
                let coord = [rect.right - T::one(), *top];
                *top += T::one();
                if *top >= rect.bottom {
                    if rect.width() == T::one() {
                        *self = BorderIter::Done;
                    } else {
                        *self = BorderIter::Bottom(rect, rect.right - T::one() - T::one());
                    }
                }
                Some(coord)
            }
            BorderIter::Bottom(rect, left) => {
                let coord = [*left, rect.bottom - T::one()];
                if *left <= rect.left {
                    *self = BorderIter::Left(rect, rect.bottom - T::one() - T::one());
                } else {
                    *left -= T::one();
                }
                Some(coord)
            }
            BorderIter::Left(rect, top) => {
                let coord = [rect.left, *top];
                *top -= T::one();
                if *top <= rect.top {
                    *self = BorderIter::Done;
                }
                Some(coord)
            }
            BorderIter::Done => None,
        }
    }
}

pub struct NumRangeIter<T: NumAssignRef + Clone + PartialOrd> {
    current: T,
    to: T,
    step: T,
    ascending: bool,
    end_inclusive: bool,
}

impl<T: NumAssignRef + Clone + PartialOrd> NumRangeIter<T> {
    pub fn new(from: T, to: T, step: T, end_inclusive: bool) -> Self {
        let ascending = from <= to;
        NumRangeIter {
            current: from,
            to,
            step,
            ascending,
            end_inclusive,
        }
    }
}

impl<T: NumAssignRef + Clone + PartialOrd> Iterator for NumRangeIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.ascending && self.current > self.to)
            || (!self.ascending && self.current < self.to)
            || (!self.end_inclusive && self.current == self.to)
        {
            None
        } else {
            let value = self.current.clone();
            if self.ascending {
                self.current += self.step.clone();
            } else {
                self.current -= self.step.clone();
            }
            Some(value)
        }
    }
}

#[cfg(test)]
mod test {
    use super::NumRect;

    #[test]
    fn test_normal_border_iter() {
        // 3x3 square
        let rect = NumRect::new(1, 1, 4, 4);
        let border: Vec<_> = rect.border().collect();
        assert!(border.contains(&[1, 1]));
        assert!(border.contains(&[2, 1]));
        assert!(border.contains(&[3, 1]));
        assert!(border.contains(&[3, 2]));
        assert!(border.contains(&[3, 3]));
        assert!(border.contains(&[2, 3]));
        assert!(border.contains(&[1, 3]));
        assert!(border.contains(&[1, 2]));
        assert_eq!(border.len(), 8);
    }

    #[test]
    fn test_short_rect() {
        // 1x3 rectangle
        let rect = NumRect::new(1, 1, 2, 4);
        let border: Vec<_> = rect.border().collect();
        assert!(border.contains(&[1, 1]));
        assert!(border.contains(&[1, 2]));
        assert!(border.contains(&[1, 3]));
        assert_eq!(border.len(), 3);
    }

    #[test]
    fn test_thin_rect() {
        // 1x3 rectangle
        let rect = NumRect::new(1, 1, 4, 2);
        let border: Vec<_> = rect.border().collect();
        assert!(border.contains(&[1, 1]));
        assert!(border.contains(&[2, 1]));
        assert!(border.contains(&[3, 1]));
        assert_eq!(border.len(), 3);
    }

    #[test]
    fn test_empty_rect() {
        // 0x0 rectangle
        let rect = NumRect::new(1, 1, 1, 1);
        let border: Vec<_> = rect.border().collect();
        assert!(border.is_empty());
    }
}
