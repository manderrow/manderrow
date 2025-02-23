use std::iter::FusedIterator;

pub struct FlattenSlice<'a, T: 'a>
where
    &'a T: IntoIterator,
{
    len: usize,
    iter: std::iter::Flatten<std::slice::Iter<'a, T>>,
}

impl<'a, T> FlattenSlice<'a, T>
where
    &'a T: IntoIterator,
{
    pub fn new(slice: &'a [T]) -> Self {
        Self {
            len: slice.iter().map(|t| t.into_iter().count()).sum(),
            iter: slice.iter().flatten(),
        }
    }
}

impl<'a, T> Clone for FlattenSlice<'a, T>
where
    &'a T: IntoIterator<IntoIter: Clone>,
{
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T> Iterator for FlattenSlice<'a, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for FlattenSlice<'a, T> where &'a T: IntoIterator {}
impl<'a, T> FusedIterator for FlattenSlice<'a, T> where &'a T: IntoIterator {}
