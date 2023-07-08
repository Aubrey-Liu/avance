#![warn(missing_docs)]

//! avance is a rust library that helps you easily report progress in
//! command line applications. It supports tracing progress in concurrent programs, and
//! also offers various utilities for customizing a progress bar.
//!
//! avance means advance or progress in spanish. This naming was inspired by
//! [tqdm](https://github.com/tqdm/tqdm), which was named after an arabic word.
//!
//! Here is an example of using avance in multiple threads:
//!
//! <img src="https://github.com/Aubrey-Liu/avance/raw/main/screenshots/multi.gif">
//!
//! # Platform support
//!
//! * Linux
//! * macOS
//! * Windows
//!
//! # Progress Bar
//!
//! [`AvanceBar`] satisfies common usage of tracing progress. It can display necessary
//! progress statistics, and can be used in the bounded or unbounded way.
//!
//! ```
//! use avance::AvanceBar;
//!
//! let pb = AvanceBar::new(100);
//! for _ in 0..100 {
//!     // ...
//!     pb.inc();
//! }
//! // Don't need to close a bar manually. It will close automatically when being dropped.
//! ```
//!
//! You're able to adjust the width, style and many other configs of a progress bar.
//! ```
//! use avance::{AvanceBar, Style};
//!
//! let pb = AvanceBar::new(100)
//!     .with_style(Style::Balloon)
//!     .with_width(80)
//!     .with_desc("avance");
//!
//! // Use a progress bar along with an iterator, eliminating the need for invoking inc or update.
//! for _ in pb.with_iter(0..100) {
//!     // ...
//! }
//! ```
//!
//! ## Behaviors:
//! - A progress bar will refresh when:
//!   - [`new`](AvanceBar::new) or [`close`](AvanceBar::close)
//!   - [`inc`](AvanceBar::inc) or [`update`](AvanceBar::update)
//!   - configuration changes (such as changing its style or width)
//! - If a progress bar's width is too large, environment width will be used instead.
//! - A progress bar can be **shared among threads fearlessly**.
//!
//! # Iterator
//!
//! Progress bar can also be associated with an iterator.
//!
//! ```
//! use avance::{AvanceIterator, Style};
//!
//! for _ in (0..100).avance().with_style(Style::ASCII).with_width(80) {
//!     // ...
//! }
//!
//! // avance provides the flexibility of changing a progress bar when iterating
//! for (_, pb) in (0..100).avance().with_pb() {
//!     // ...
//!     pb.set_postfix("");
//! }
//! ```
//!
//! # Style
//!
//! avance provides a range of pre-definded progress styles (at [`Style`]),
//! and also allows users to easily **customize** the style according to their preferences.
//!
//! ```
//! # use avance::AvanceIterator;
//! for _ in (0..1000).avance().with_style_str("=>-") {
//!     // ...
//! }
//! ```
//!
//! # TODOs:
//! - [ ] A progress bar for io pipes
//! - [ ] A Monitor for very slow progress bars

pub mod bar;
pub mod iter;
pub mod style;

#[doc(inline)]
pub use bar::{set_max_progress_bars, AvanceBar};
#[doc(inline)]
pub use iter::{AvanceBarIter, AvanceIter, AvanceIterator};
#[doc(inline)]
pub use style::Style;
