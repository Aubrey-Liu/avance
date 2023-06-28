//! avance is a rust library that helps you easily report progress in
//! command line applications. It supports tracing progress in concurrent programs, and
//! also offers various utilities for formatting progress bars.
//!
//! avance means advance or progress in spanish. This naming was inspired by
//! [tqdm](https://github.com/tqdm/tqdm), which was named after an arabic word.
//!
//! # Platform support
//!
//! * Linux
//! * macOS
//! * Windows
//!
//! # Progress Bar
//! [`AvanceBar`] satisfies common usage of tracing progress. It can display necessary
//! progress statistics, and can be used in the bounded or unbounded way.
//!
//! ```
//! use avance::bar::AvanceBar;
//!
//! let bar = AvanceBar::new(100);
//! for _ in 0..100 {
//!     bar.update(1);
//!     // do something here
//! }
//! // Don't need to close a bar manually. It will close automatically when being dropped.
//! ```
//!
//! You're able to adjust the width, style and other attributes of the progress bar.
//! ```
//! use avance::AvanceBar;
//! use avance::Style;
//!
//! let bar = AvanceBar::new(100);
//! bar.set_style(Style::Balloon);
//! bar.set_width(80);
//! bar.set_description("avance");
//! ```
//!
//! Behaviors:
//! * A progress bar will refresh when (1) created (2) `set_*` or `update` are called (3) closed
//! * If width is too large, it will be adjusted to environment width
//! * A progress bar can be safely shared among threads, without damaging the display.
//!
//! # Iterator
//!
//! Progress bar can also be associated with an iterator.
//!
//! ```
//! use avance::AvanceIterator;
//! use avance::Style;
//!
//! // methods can be chained
//! for _ in (0..100).avance().style(Style::ASCII).width(80) {
//!     // do something here
//! }
//! ```

pub mod bar;
pub mod iter;
pub mod style;

pub use bar::AvanceBar;
pub use iter::AvanceIterator;
pub use style::Style;
