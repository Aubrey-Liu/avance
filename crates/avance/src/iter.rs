//! A wrapped iterator that shows progress

use std::borrow::Cow;

use super::*;

/// An iterator wrapper that shows a progress bar
pub struct AvanceIter<Iter> {
    pub(crate) iter: Iter,
    pub(crate) bar: AvanceBar,
}

/// Wrap an iterator to display its progress
pub trait AvanceIterator
where
    Self: Sized + Iterator,
{
    /// Wrap an iterator to display its progress, using the upper hound
    /// of iterator's size as the total length of the progress bar.
    ///
    /// See another way of progressing with an iterator at [`AvanceBar::with_iter`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use avance::AvanceIterator;
    /// for _ in (0..1000).avance() {
    ///     // ...
    /// }
    /// ```
    fn avance(self) -> AvanceIter<Self> {
        AvanceIter {
            bar: AvanceBar::with_hint(self.size_hint().1),
            iter: self,
        }
    }
}

impl<Iter: Iterator> AvanceIter<Iter> {
    /// Set the style of a progress bar.
    ///
    /// See [AvanceBar::with_style]
    ///
    /// # Examples
    ///
    /// ```
    /// # use avance::{AvanceIterator, Style};
    /// for _ in (0..1000).avance().with_style(Style::Balloon) {
    ///     // ...
    /// }
    /// ```
    pub fn with_style(self, style: Style) -> Self {
        self.bar.set_style(style);
        self
    }

    /// Set the user-custom style of a progress bar.
    ///
    /// See [AvanceBar::with_style_str]
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceIterator;
    /// for _ in (0..1000).avance().with_style_str("=>-") {
    ///     // ...
    /// }
    /// ```
    pub fn with_style_str(self, s: &'static str) -> Self {
        self.bar.set_style_str(s);
        self
    }

    /// Set the description of a progress bar.
    ///
    /// See [AvanceBar::with_desc]
    ///
    /// # Examples
    ///
    /// ```
    /// # use avance::AvanceIterator;
    /// for _ in (0..1000).avance().with_desc("task name") {
    ///     // ...
    /// }
    /// ```
    pub fn with_desc(self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.bar.set_desc(desc);
        self
    }

    /// Displaying numbers in a human readable format, using SI metric prefix
    /// (k = 10^3, M = 10^6, etc.)
    pub fn with_unit_scale(self, unit_scale: bool) -> Self {
        self.bar.set_unit_scale(unit_scale);
        self
    }

    /// Set a progress bar's width
    ///
    /// See [AvanceBar::with_width]
    ///
    /// # Examples
    ///
    /// ```
    /// # use avance::AvanceIterator;
    /// for _ in (0..1000).avance().with_width(80) {
    ///     // ...
    /// }
    /// ```
    pub fn with_width(self, width: u16) -> Self {
        self.bar.set_width(width);
        self
    }

    /// Creates an iterator which gives the original item and a progress bar handler.
    ///
    /// Useful when you use the iterator-style progress bar, and meanwhile want to
    /// control the progress bar when iterating (such as setting the postfix).
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceIterator;
    /// // Configurate before calling with_pb
    /// for (_, pb) in (0..1000).avance().with_width(80).with_pb() {
    ///     // ...
    ///     pb.set_postfix("");
    /// }
    /// ```
    pub fn with_pb(self) -> AvanceBarIter<Iter> {
        AvanceBarIter(self)
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

/// Wraps an AvanceIter and gives a progress bar handler when iterating.
///
/// You don't have to call [`inc`](AvanceBar::inc) or [`update`](AvanceBar::update)
/// explicitly when using an AvanceBarIter.
pub struct AvanceBarIter<Iter>(AvanceIter<Iter>);

impl<Iter: Iterator> Iterator for AvanceBarIter<Iter> {
    type Item = (Iter::Item, AvanceBar);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|item| (item, self.0.bar.clone()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<Iter: ExactSizeIterator> ExactSizeIterator for AvanceBarIter<Iter> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<Iter: DoubleEndedIterator> DoubleEndedIterator for AvanceBarIter<Iter> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|item| (item, self.0.bar.clone()))
    }
}
