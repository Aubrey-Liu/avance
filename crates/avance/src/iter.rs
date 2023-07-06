//! A wrapped iterator that shows progress

use std::ops::{Deref, DerefMut};

use crate::{bar::AvanceBar, Style};

/// An iterator wrapper that shows a progress bar
pub struct AvanceIter<Iter> {
    pub(crate) iter: Iter,
    pub(crate) bar: AvanceBar,
}

/// Wraps an iterator to display its progress
pub trait AvanceIterator
where
    Self: Sized + Iterator,
{
    /// Wraps an iterator to display its progress, using the upper hound
    /// of iterator's size as the total length of the progress bar.
    ///
    /// See other ways of progressing with an iterator at:
    /// - [`avance`]
    /// - [`AvanceBar::with_iter`]
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::AvanceIterator;
    ///
    /// for _ in (0..1000).avance() {
    ///     // do something here
    /// }
    /// ```
    fn avance(self) -> AvanceIter<Self> {
        avance(self)
    }
}

impl<Iter: Iterator> AvanceIter<Iter> {
    /// Set the style of a progress bar.
    ///
    /// See [AvanceBar::set_style]
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().with_style(Style::Balloon) {
    ///     // do something here
    /// }
    /// ```
    pub fn with_style(self, style: Style) -> Self {
        self.bar.set_style(style);
        self
    }

    /// Set the description of a progress bar.
    ///
    /// See [AvanceBar::set_desc].
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().with_desc("task name") {
    ///     // do something here
    /// }
    /// ```
    pub fn with_desc(self, desc: impl ToString) -> Self {
        self.bar.set_desc(desc);
        self
    }

    /// Set a progress bar's width
    ///
    /// See [AvanceBar::set_width].
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().with_width(80) {
    ///     // do something here
    /// }
    /// ```
    pub fn with_width(self, width: u16) -> Self {
        self.bar.set_width(width);
        self
    }
}

// Implement AcanceIterator trait for all Iterator types
impl<Iter: Iterator> AvanceIterator for Iter {}

impl<Iter: Iterator> Iterator for AvanceIter<Iter> {
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            self.bar.inc();
            Some(next)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<Iter: ExactSizeIterator> ExactSizeIterator for AvanceIter<Iter> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<Iter: DoubleEndedIterator> DoubleEndedIterator for AvanceIter<Iter> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next_back() {
            self.bar.inc();
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

/// Wraps an iterator to display a progress bar.
///     
/// See other ways of progressing with iterators at:
/// - [`AvanceIterator`]
/// - [`AvanceBar::with_iter`]
///
/// # Examples
/// ```
/// use avance::*;
///
/// for _ in avance(0..1000) {
///     // do something here
/// }
/// ```
pub fn avance<Iter: Iterator>(iter: Iter) -> AvanceIter<Iter> {
    AvanceIter {
        bar: AvanceBar::with_hint(iter.size_hint().1),
        iter,
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::{AvanceIterator, Style};

    #[test]
    fn avance_iter() {
        for _ in (0..100).avance() {
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn associated_methods() {
        for _ in (0..100)
            .avance()
            .with_style(Style::Block)
            .with_desc("avance")
            .with_width(85)
        {
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn infinity() {
        for i in (0..).avance() {
            if i > 200 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}
