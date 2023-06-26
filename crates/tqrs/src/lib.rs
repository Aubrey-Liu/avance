//! Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).
//!
//! From original documentation:
//! > tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
//! > Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
//!
//! This crate provides a wrapper [Iterator]. It controls multiple progress bars when `next` is called.
//! Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.
//!

use std::*;

use std::io::Write;
use std::ops::{Deref, DerefMut};

use crossterm::QueueableCommand;

#[cfg(test)]
mod test;

pub mod style;
pub use style::Style;

/* -------------------------------------------------------------------------- */
/*                                    TQDM                                    */
/* -------------------------------------------------------------------------- */

/* -------------------------------- FUNCTION -------------------------------- */

///
///
/// Panics

pub fn tqrs<Item, Iter: Iterator<Item = Item>>(iterable: Iter) -> ProgressBar<Item, Iter> {
    let id = ID.fetch_add(1, sync::atomic::Ordering::SeqCst);

    let mut bars = bars().lock().unwrap();
    bars.insert(
        id,
        Info {
            begin: time::SystemTime::now(),
            config: Config::default(),

            nitem: 0usize,
            total: iterable.size_hint().1,
        },
    );

    drop(refresh());

    ProgressBar {
        iterable,
        id,

        next: time::UNIX_EPOCH,
        step: 0usize,
        freqlim: 24.,
    }
}

/// Manually refresh all progress bars

pub fn refresh() -> io::Result<()> {
    let mut output = io::stderr();

    let bars = bars().lock().unwrap();
    let (ncols, _nrows) = terminal_size();

    let n = bars.len();
    if bars.is_empty() {
        return Ok(());
    }

    output.queue(crossterm::cursor::Hide)?;
    output.queue(crossterm::cursor::MoveToColumn(0))?;

    for info in bars.values() {
        let bar = format!("{:<1$}", format!("{}", info), ncols);
        output.queue(crossterm::style::Print(bar))?;
    }

    if let Some(rows) = num::NonZeroUsize::new(n - 1) {
        output.queue(crossterm::cursor::MoveUp(rows.get() as u16))?;
    }

    output.queue(crossterm::cursor::Show)?;
    output.flush()
}

/* --------------------------------- STRUCT --------------------------------- */

/// Iterator wrapper that updates progress bar on `next`
///
///
/// ## Examples
///
/// - Basic Usage
/// ```
/// for _ in tqdm(0..100) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Composition
/// ```
/// for _ in tqdm(tqdm(0..100).take(50)) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Multi-threading
/// ```
/// let threads: Vec<_> = [200, 400, 100].iter().map(|its| {
///         std::thread::spawn(move || {
///             for _ in tqdm(0..*its) {
///                 thread::sleep(Duration::from_millis(10));
///             }
///         })
///     })
///     .collect();
///
/// for handle in threads {
///     handle.join().unwrap();
/// }
/// ```

pub struct ProgressBar<Item, Iter: Iterator<Item = Item>> {
    /// Iterable wrapped
    pub iterable: Iter,

    /// Hashed
    id: usize,

    /// Next refresh time
    next: time::SystemTime,

    /// Cached updates
    step: usize,

    /// Refresh frequency
    freqlim: f64,
}

impl<Item, Iter: Iterator<Item = Item>> ProgressBar<Item, Iter> {
    pub fn desc<S: ToString>(self, desc: Option<S>) -> Self {
        if let Ok(mut tqdm) = bars().lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.desc = desc.map(|desc| desc.to_string());
            }
        }

        self
    }

    pub fn width(self, width: Option<usize>) -> Self {
        if let Ok(mut tqdm) = bars().lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.width = width;
            }
        }

        self
    }

    pub fn style(self, style: Style) -> Self {
        if let Ok(mut tqdm) = bars().lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.style = style;
            }
        }

        self
    }
}

impl<Item, Iter: Iterator<Item = Item>> ProgressBar<Item, Iter> {
    pub fn close(&mut self) -> io::Result<()> {
        if let Ok(mut tqdm) = bars().lock() {
            let mut info = tqdm.remove(&self.id).unwrap();
            info.nitem += self.step;

            io::stderr().queue(crossterm::cursor::MoveToColumn(0))?;
            io::stderr().queue(crossterm::style::Print(format!("{}\n", info)))?;
        }

        refresh()
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for ProgressBar<Item, Iter> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.elapsed().is_ok() {
            if let Ok(mut tqdm) = bars().lock() {
                let info = tqdm.get_mut(&self.id).unwrap();

                info.nitem += self.step;
                self.step = 0;
            }

            drop(refresh());

            self.next = time::SystemTime::now();
            self.next += time::Duration::from_secs_f64(1. / self.freqlim);
        }

        if let Some(next) = self.iterable.next() {
            self.step += 1;
            Some(next)
        } else {
            None
        }
    }
}

impl<Item, Iter: Iterator<Item = Item>> Deref for ProgressBar<Item, Iter> {
    type Target = Iter;

    fn deref(&self) -> &Self::Target {
        &self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> DerefMut for ProgressBar<Item, Iter> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> Drop for ProgressBar<Item, Iter> {
    fn drop(&mut self) {
        drop(self.close());
    }
}

/* ---------------------------------- TRAIT --------------------------------- */

/// Trait that allows `.tqdm()` method chaining, equivalent to `tqdm::tqdm(iter)`
///
///
/// ## Examples
/// ```
/// use tqdm::Iter;
/// (0..).take(1000).tqdm()
/// ```

pub trait Iter<Item>: Iterator<Item = Item> {
    fn tqrs(self) -> ProgressBar<Item, Self>
    where
        Self: Sized,
    {
        tqrs(self)
    }
}

impl<Item, Iter: Iterator<Item = Item>> crate::Iter<Item> for Iter {}

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

/* --------------------------------- STATIC --------------------------------- */

static ID: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(0);
static BARS: sync::OnceLock<sync::Mutex<collections::HashMap<usize, Info>>> = sync::OnceLock::new();

fn terminal_size<T: From<u16>>() -> (T, T) {
    let (width, height) = crossterm::terminal::size().unwrap_or((80, 64));
    (T::from(width), T::from(height))
}

fn bars() -> &'static sync::Mutex<collections::HashMap<usize, Info>> {
    BARS.get_or_init(|| sync::Mutex::new(collections::HashMap::new()))
}

/* --------------------------------- CONFIG --------------------------------- */

#[derive(Default)]

pub struct Config {
    pub desc: Option<String>,
    pub width: Option<usize>,
    pub style: style::Style,
}

/* ---------------------------------- INFO ---------------------------------- */

struct Info {
    begin: time::SystemTime,
    config: Config,

    nitem: usize,
    total: Option<usize>,
}

impl fmt::Display for Info {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed = {
            let time = self.begin.elapsed();
            time.as_ref().map_or(0., time::Duration::as_secs_f64)
        };

        let Config { desc, width, style } = &self.config;
        let desc = desc.clone().map_or(String::new(), |desc| desc + ": ");
        let width = width.unwrap_or_else(|| terminal_size().0);

        /// Time format omitting leading 0
        fn ftime(seconds: usize) -> String {
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            match seconds / 3600 {
                0 => format!("{:02}:{:02}", m, s),
                h => format!("{:02}:{:02}:{:02}", h, m, s),
            }
        }

        let it = self.nitem;
        let its = it as f64 / elapsed;
        let time = ftime(elapsed as usize);

        match self.total {
            None => fmt.write_fmt(format_args!("{}{}it [{}, {:.02}it/s]", desc, it, time, its)),

            Some(total) => {
                let pct = (it as f64 / total as f64).clamp(0.0, 1.0);
                let eta = match it {
                    0 => String::from("?"),
                    _ => ftime((elapsed / pct * (1. - pct)) as usize),
                };

                let bra_ = format!("{}{:>3}%|", desc, (100.0 * pct) as usize);
                let _ket = format!("| {}/{} [{}<{}, {:.02}it/s]", it, total, time, eta, its);
                let tqdm = {
                    let limit = width.saturating_sub(bra_.len() + _ket.len());
                    let pattern: Vec<_> = style.to_string().chars().collect();

                    let m = pattern.len();
                    let n = ((limit as f64 * pct) * m as f64) as usize;

                    let bar = pattern.last().unwrap().to_string().repeat(n / m);
                    match n / m {
                        x if x == limit => bar,
                        _ => format!("{:<limit$}", format!("{}{}", bar, pattern[n % m])),
                    }
                };

                fmt.write_fmt(format_args!("{}{}{}", bra_, tqdm, _ket))
            }
        }
    }
}
