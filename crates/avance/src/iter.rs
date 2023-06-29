//! A wrapped iterator that shows progress

use std::ops::{Deref, DerefMut};

use crate::{bar::AvanceBar, Style};

/// An iterator wrapper that shows a progress bar
pub struct AvanceIter<Iter: Iterator> {
    iter: Iter,
    bar: AvanceBar,
}

pub trait AvanceIterator
where
    Self: Sized + Iterator,
{
    /// Wraps an iterator to display a progress bar.
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
        AvanceIter {
            bar: AvanceBar::with_hint(self.size_hint().1),
            iter: self,
        }
    }
}

impl<Iter: Iterator> AvanceIter<Iter> {
    /// Set the style of a progress bar.
    ///
    /// See available styles in [`Style`]
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().style(Style::Balloon) {
    ///     // do something here
    /// }
    /// ```
    pub fn style(self, style: Style) -> Self {
        self.bar.set_style(style);
        self
    }

    /// Set the description of a progress bar.
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().desc("task name") {
    ///     // do something here
    /// }
    /// ```
    pub fn desc(self, desc: impl ToString) -> Self {
        self.bar.set_description(desc);
        self
    }

    /// Set a progress bar's width
    ///
    /// If width is larger than terminal width, progress bar will adjust
    /// to the terminal width.
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceIterator, Style};
    ///
    /// for _ in (0..1000).avance().width(80) {
    ///     // do something here
    /// }
    /// ```
    pub fn width(self, width: u16) -> Self {
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

/// Wraps an iterator to display a progress bar.
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
            .style(Style::Block)
            .desc("avance")
            .width(85)
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
            thread::sleep(Duration::from_millis(20));
        }
    }
}
