pub trait OrdExt {
    fn at_least(self, other: Self) -> Self;
    fn at_most(self, other: Self) -> Self;
}

impl<T: PartialOrd> OrdExt for T {
    #[allow(clippy::disallowed_methods)]
    #[inline]
    fn at_least(self, other: Self) -> Self {
        if other < self { self } else { other }
    }

    #[allow(clippy::disallowed_methods)]
    #[inline]
    fn at_most(self, other: Self) -> Self {
        if other > self { self } else { other }
    }
}
