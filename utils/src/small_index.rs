use std::marker::PhantomData;
use std::ops::{Add, Index, IndexMut};
use typenum::{B1, Const, IsGreater, IsLessOrEqual, ToUInt, U0, Unsigned};

pub type SmallIndex<const LIMIT: usize> = U8Index<<Const<LIMIT> as ToUInt>::Output>;

#[repr(transparent)]
pub struct U8Index<LIMIT: Unsigned>(u8, PhantomData<LIMIT>);

impl<LIMIT: Unsigned> U8Index<LIMIT> {
    #[must_use]
    #[inline]
    pub fn try_new(index: u8) -> Option<Self> {
        if index < LIMIT::U8 {
            Some(Self(index, PhantomData))
        } else {
            None
        }
    }

    /// # SAFETY
    /// The caller must ensure that `index` is less than `LIMIT`
    /// otherwise indexing will be out of bounds
    #[must_use]
    #[inline]
    pub unsafe fn new_unchecked(index: usize) -> Self {
        debug_assert!(index < LIMIT::USIZE);
        Self(index as u8, PhantomData)
    }

    #[must_use]
    #[inline]
    pub fn get(&self) -> usize {
        self.0 as usize
    }

    #[must_use]
    #[inline]
    pub fn increment_if(&self, condition: bool) -> U8Index<<LIMIT as Add<B1>>::Output>
    where
        LIMIT: Add<B1, Output: Unsigned>,
    {
        if condition {
            U8Index(self.0.wrapping_add(1), PhantomData)
        } else {
            U8Index(self.0, PhantomData)
        }
    }

    #[must_use]
    #[inline]
    pub fn zero() -> Self
    where
        LIMIT: IsGreater<U0, Output = B1>,
    {
        U8Index(0, PhantomData)
    }

    #[must_use]
    #[inline]
    pub fn max() -> Self
    where
        LIMIT: IsGreater<U0, Output = B1>,
    {
        U8Index(LIMIT::to_u8() - 1, PhantomData)
    }
}

impl<T, N: Unsigned, const LIMIT: usize> Index<U8Index<N>> for [T; LIMIT]
where
    Const<LIMIT>: ToUInt<Output: Unsigned>,
    N: IsLessOrEqual<<Const<LIMIT> as ToUInt>::Output, Output = B1>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: U8Index<N>) -> &Self::Output {
        // SAFETY: The index is guaranteed to be smaller than LIMIT
        unsafe { self.get_unchecked(index.0 as usize) }
    }
}

impl<T, N: Unsigned, const LIMIT: usize> IndexMut<U8Index<N>> for [T; LIMIT]
where
    Const<LIMIT>: ToUInt<Output: Unsigned>,
    N: IsLessOrEqual<<Const<LIMIT> as ToUInt>::Output>,
    N: IsLessOrEqual<<Const<LIMIT> as ToUInt>::Output, Output = B1>,
{
    #[inline]
    fn index_mut(&mut self, index: U8Index<N>) -> &mut Self::Output {
        // SAFETY: The index is guaranteed to be smaller than LIMIT
        unsafe { self.get_unchecked_mut(index.0 as usize) }
    }
}
