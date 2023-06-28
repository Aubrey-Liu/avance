//! TODO: documentation

use std::ops::{Deref, DerefMut};

use crate::bar::AvanceBar;

pub struct AvanceIter<Iter: Iterator> {
    iter: Iter,
    bar: AvanceBar,
}

pub trait AvanceIterator
where
    Self: Sized + Iterator,
{
    fn avance(self) -> AvanceIter<Self> {
        AvanceIter {
            bar: AvanceBar::with_hint(self.size_hint().1),
            iter: self,
        }
    }
}

impl<Iter: Iterator> AvanceIterator for Iter {}

impl<Iter: Iterator> Iterator for AvanceIter<Iter> {
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            self.bar.update(1);
            Some(next)
        } else {
            None
        }
    }
}

impl<Iter: Iterator> Deref for AvanceIter<Iter> {
    type Target = Iter;

    fn deref(&self) -> &Self::Target {
        &self.iter
    }
}

impl<Iter: Iterator> DerefMut for AvanceIter<Iter> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.iter
    }
}

pub fn avance<Iter: Iterator>(iter: Iter) -> AvanceIter<Iter> {
    AvanceIter {
        bar: AvanceBar::with_hint(iter.size_hint().1),
        iter,
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::AvanceIterator;

    #[test]
    fn avance_iter() {
        for _ in (0..100).avance() {
            thread::sleep(Duration::from_millis(20));
        }
    }
}
