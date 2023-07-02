//! avance is a rust library that helps you easily report progress in
//! command line applications. It supports tracing progress in concurrent programs, and
//! also offers various utilities for formatting progress bars.
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
//! [`AvanceBar`] satisfies common usage of tracing progress. It can display necessary
//! progress statistics, and can be used in the bounded or unbounded way.
//!
//! ```
//! use avance::bar::AvanceBar;
//!
//! let pb = AvanceBar::new(100);
//! for _ in 0..100 {
//!     pb.inc();
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
//! let pb = AvanceBar::new(100)
//!     .with_style(Style::Balloon)
//!     .with_width(80)
//!     .with_desc("avance");
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
//!     // ...
//! }
//!
//! // also supports build a progress bar like this
//! use avance::avance;
//! for _ in avance(0..100).style(Style::Balloon).desc("avance") {
//!     // ...
//! }
//! ```
//!
//! # TODOs:
//! - [ ] Support user-defined progress bar style
//! - [ ] Implement Read & Write traits for [`AvanceIter`](iter::AvanceIter)

pub mod bar;
pub mod iter;
pub mod style;

pub use bar::{set_max_progress_bars, AvanceBar};
pub use iter::{avance, AvanceIterator};
pub use style::Style;
