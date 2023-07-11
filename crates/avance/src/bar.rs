//! A progress bar and all utilities.

use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::tty::IsTty;
use crossterm::QueueableCommand;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{stderr, Result, Write};
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

#[cfg(has_std_once_cell = "false")]
use once_cell::sync::OnceCell as OnceLock;
#[cfg(has_std_once_cell = "true")]
use std::sync::OnceLock;

use super::*;

/// The progress bar
#[derive(Debug, Clone)]
pub struct AvanceBar {
    state: AtomicState,
}

// Public Interface
impl AvanceBar {
    /// Create a new progress bar
    pub fn new(total: u64) -> Self {
        let pb = AvanceBar {
            state: Arc::new(Mutex::new(State::new(Some(total)))),
        };
        pb.refresh();
        pb
    }

    /// Wrap an iterator to display its progress.
    ///
    /// See another way of progressing with an iterator at [`AvancesIterator`](crate::AvanceIterator)
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
    /// let pb = AvanceBar::new(100);
    /// for _ in pb.with_iter(0..100) {
    ///     // ...
    /// }
    /// ```
    pub fn with_iter<Iter: Iterator>(self, iter: Iter) -> AvanceIter<Iter> {
        AvanceIter { iter, bar: self }
    }

    /// Builder-like function for a progress bar with a given style
    /// (default: [`Style::ASCII`]).
    ///
    /// See available styles in [`Style`]
    ///
    /// # Examples
    /// ```
    /// # use avance::{AvanceBar, Style};
    /// let pb = AvanceBar::new(1000).with_style(Style::Block);
    /// ```
    pub fn with_style(self, style: Style) -> Self {
        self.set_style(style);
        self
    }

    /// Builder-like function for a progress bar with user custom style
    ///
    /// A custom style string is like `|{Finished}{Current}{ToDo}|`:
    /// - Finished & ToDo: One character
    /// - Current: One to many characters
    ///
    /// Take `"#0123456789 "` as an example, the presentation of the bar will be like:
    /// `|######3      |`
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
    /// let pb = AvanceBar::new(1000).with_style_str("=>-");
    /// ```
    pub fn with_style_str(self, s: impl ToString) -> Self {
        self.set_style_str(s);
        self
    }

    /// Builder-like function for a progress bar with width
    ///
    /// If width is larger than terminal width, progress bar will adjust
    /// to the terminal width.
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
    /// let pb = AvanceBar::new(1000).with_width(80);
    /// ```
    pub fn with_width(self, width: u16) -> Self {
        self.set_width(width);
        self
    }

    /// Builder-like function for a progress bar with description
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
    /// let pb = AvanceBar::new(1000).with_desc("my task");
    /// ```
    pub fn with_desc(self, desc: impl ToString) -> Self {
        self.set_desc(desc);
        self
    }

    /// Build a new progress bar with configs of another progress bar.
    /// Only the configs and length of the old progress bar will be retained.
    ///
    /// # Examples
    /// ```     
    /// # use avance::{AvanceBar, Style};  
    /// let pb1 = AvanceBar::new(100)
    ///     .with_style(Style::Balloon)
    ///     .with_width(90)
    ///     .with_desc("task1");
    /// // Reuse the style and width of pb1, but
    /// // change the description and length.
    /// let pb2 = AvanceBar::with_config_of(&pb1)
    ///     .with_total(200)
    ///     .with_desc("task2");
    /// ```
    pub fn with_config_of(pb: &AvanceBar) -> Self {
        let new_state = pb.state.lock().unwrap().clone();
        let new_pb = AvanceBar {
            state: Arc::new(Mutex::new(new_state)),
        };
        new_pb.refresh();
        new_pb
    }

    /// Builder-like function for a progress bar with minimun
    /// update interval (default: 1/15) seconds.
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
    /// let pb = AvanceBar::new(1000).with_interval(1.0 / 5.0);
    /// ```
    pub fn with_interval(self, interval: f64) -> Self {
        self.set_interval(interval);
        self
    }

    /// Builder-like function for a progress bar with length
    ///
    /// Useful when you reuse some configs of another progress bar,
    /// but want to change the length.
    ///
    /// # Examples
    /// ```
    /// # use avance::{AvanceBar, Style};
    /// let pb1 = AvanceBar::new(100)
    ///    .with_style(Style::Balloon)
    ///    .with_width(90);
    /// // Reuse pb1's config, but override the length.
    /// let pb2 = AvanceBar::with_config_of(&pb1).with_total(200);
    /// ```
    pub fn with_total(self, total: u64) -> Self {
        self.set_total(total);
        self
    }

    /// Override the postfix of a progress bar.
    ///
    /// Postfix is usually used for **dynamically** displaying some
    /// additional information, such as the accuracy when training a model.
    ///
    /// See [`AvanceIter::with_pb`] if you want to change the postfix when
    /// progressing with an iterator.
    pub fn set_postfix(&self, postfix: impl ToString) {
        let mut state = self.state.lock().unwrap();
        state.config.postfix = Some(postfix.to_string());
        let _ = state.draw(None);
    }

    /// Advance the progress bar by n steps.
    pub fn update(&self, n: u64) {
        let mut state = self.state.lock().unwrap();

        state.update(n);
    }

    /// Advance the progress bar by one step, with the same effect as
    /// [`update(1)`](AvanceBar::update)
    ///
    /// # Examples
    /// ```
    /// use avance::AvanceBar;
    ///
    /// let pb = AvanceBar::new(1000);
    /// for _ in 0..1000 {
    ///     // ...
    ///     pb.inc();
    /// }
    /// ```
    pub fn inc(&self) {
        self.update(1);
    }

    /// Manually stop the progress bar, and leave the current progress on terminal.
    /// Usually users don't have to call this method directly, as a progress bar will
    /// be closed automatically when dropped.
    ///
    /// Users should close a bar manually when they want to preserve the rendering order
    /// of progress bars, otherwise, progress bars will be closed in the order of being
    /// dropped (Closing order is the same as the rendering order).
    pub fn close(&self) {
        let _ = self.state.lock().unwrap().close();
    }

    /// Set the style (default: [`Style::ASCII`]) of a progress bar.
    pub fn set_style(&self, style: Style) {
        let mut state = self.state.lock().unwrap();
        state.config.style = style;
        let _ = state.draw(None);
    }

    /// Set the user-custom style of a progress bar.
    pub fn set_style_str(&self, s: impl ToString) {
        let mut state = self.state.lock().unwrap();
        state.config.style = Style::Custom(s.to_string());
        let _ = state.draw(None);
    }

    /// Set a progress bar's width
    pub fn set_width(&self, width: u16) {
        let mut state = self.state.lock().unwrap();
        state.config.width = Some(width);
        let _ = state.clear();
        let _ = state.draw(None);
    }

    /// Set the description (prefix) of a progress bar.
    pub fn set_desc(&self, desc: impl ToString) {
        let mut state = self.state.lock().unwrap();
        state.config.desc = Some(desc.to_string());
        let _ = state.draw(None);
    }

    /// Set the minimun update interval (default: 1/15) seconds.
    ///
    /// Only positive intervals are allowed. Uses the default interval (1/15s)
    /// when the argument is invalid.
    pub fn set_interval(&self, interval: f64) {
        let mut state = self.state.lock().unwrap();
        state.config.interval = if interval.is_sign_positive() {
            interval
        } else {
            1.0 / 15.0
        };
    }

    /// Set the length of a progress bar.
    pub fn set_total(&self, total: u64) {
        let mut state = self.state.lock().unwrap();
        state.total = Some(total);
        let _ = state.draw(None);
    }
}

// Private Interface
impl AvanceBar {
    /// Creates a progress bar from an iterator's size hint
    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(size_hint.map(|s| s as u64)))),
        }
    }

    /// Refresh the progress bar.
    fn refresh(&self) {
        let state = self.state.lock().unwrap();
        let _ = state.draw(None);
    }
}

/// The inner state of a progress bar
#[derive(Debug)]
struct State {
    config: Config,
    begin: Instant,
    last: Instant,
    id: ID,
    n: u64,
    cached: u64,
    total: Option<u64>,
}

impl State {
    /// Create a new state of a progress bar
    fn new(total: Option<u64>) -> Self {
        Self {
            config: Config::new(),
            begin: Instant::now(),
            last: Instant::now(),
            id: next_free_pos(),
            n: 0,
            cached: 0,
            total,
        }
    }

    /// Advance n steps for a progress bar
    fn update(&mut self, n: u64) {
        self.cached += n;

        if let Some(total) = self.total {
            if self.n >= total {
                return;
            }

            if self.n + self.cached >= total {
                self.force_update();
                let _ = self.draw(None);
            }
        }

        if self.last.elapsed().as_secs_f64() >= self.config.interval {
            self.n += self.cached;
            self.cached = 0;
            self.last = Instant::now();
            let _ = self.draw(None);
        }
    }

    /// Make progress without caring the interval
    fn force_update(&mut self) {
        self.n = if let Some(total) = self.total {
            min(total, self.n + self.cached)
        } else {
            self.n + self.cached
        };
        self.cached = 0;
        self.last = Instant::now();
    }

    /// Draw progress bar onto terminal
    fn draw(&self, pos: Option<u16>) -> Result<()> {
        if !self.drawable() {
            return Ok(());
        }

        let mut target = stderr().lock();
        let pos = if let Some(pos) = pos {
            pos
        } else {
            self.get_pos()
        };

        let ncols = terminal_size().0;
        let nrows = nrows();

        if pos >= nrows {
            return Ok(());
        }

        let msg = if pos == nrows - 1 {
            "... (more hidden) ...".to_string()
        } else {
            format!("{}", self)
        };
        let msg = format!("{:1$}", msg, ncols as usize);

        if pos != 0 {
            target
                .queue(Print("\n".repeat(pos as usize)))?
                .queue(Print(msg))?
                .queue(MoveUp(pos))?
                .queue(MoveToColumn(ncols))?
        } else {
            target.queue(MoveToColumn(0))?.queue(Print(msg))?
        }
        .flush()
    }

    /// Is a progress bar able to be displayed
    fn drawable(&self) -> bool {
        // is_terminal is stable on 1.70.0
        stderr().is_tty() && self.try_get_pos().is_some()
    }

    /// Sweep a progress bar from the terminal.
    /// Useful when a progress bar's width was changed.
    fn clear(&self) -> Result<()> {
        if !self.drawable() {
            return Ok(());
        }

        let mut target = stderr().lock();
        let pos = self.get_pos();
        let nrows = nrows();
        if pos >= nrows {
            return Ok(());
        }

        if pos != 0 {
            target
                .queue(Print("\n".repeat(pos as usize)))?
                .queue(Clear(ClearType::CurrentLine))?
                .queue(MoveUp(pos))?
        } else {
            target.queue(Clear(ClearType::CurrentLine))?
        }
        .flush()
    }

    fn close(&mut self) -> Result<()> {
        if !self.drawable() {
            // already closed
            return Ok(());
        }

        // Don't update when there's nothing new
        if !matches!(self.total, Some(t) if t == self.n) {
            self.force_update();
        }
        let _ = self.draw(Some(0));

        // Close the current bar and move up other bars
        reposition(self.id);

        // Move cursor to the end of the next line
        let mut target = stderr().lock();
        let ncols = terminal_size().0;

        target.queue(Print('\n'))?;
        if !is_finished() {
            // only do this when some bars are still in progress
            target.queue(MoveToColumn(ncols))?;
        }
        target.flush()
    }

    /// Try to find a progress bar's position. If none, means the bar has already been closed.
    fn try_get_pos(&self) -> Option<Pos> {
        let positions = positions().lock().unwrap();
        positions.get(&self.id).copied()
    }

    /// Get a progress bar's position, assuming the bar isn't closed.
    ///
    /// # Panics
    /// Panics if the progress bar was closed.
    fn get_pos(&self) -> Pos {
        self.try_get_pos().unwrap()
    }
}

impl Display for State {
    /// Convert a progerss bar into human readable format.
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        let elapsed = self.last.duration_since(self.begin).as_secs_f64();

        let desc = self
            .config
            .desc
            .as_ref()
            .map_or_else(String::new, |desc| format!("{}: ", desc));

        let postfix = self
            .config
            .postfix
            .as_ref()
            .map_or_else(String::new, |p| format!(", {}", p));

        let terminal_width = terminal::size().map_or(80, |(c, _)| c);
        let width = self
            .config
            .width
            .map_or(terminal_width, |w| min(w, terminal_width));

        // Time formatting function, which omits the leading 0s
        let ftime = |seconds: usize| -> String {
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            match seconds / 3600 {
                0 => format!("{:02}:{:02}", m, s),
                h => format!("{:02}:{:02}:{:02}", h, m, s),
            }
        };

        let it = self.n;
        let its = it as f64 / elapsed;
        let time = ftime(elapsed as usize);

        match self.total {
            None => fmt.write_fmt(format_args!(
                "{}{}it [{}, {:.02}it/s]{}",
                desc, it, time, its, postfix
            )),

            Some(total) => {
                let pct = (it as f64 / total as f64).clamp(0.0, 1.0);
                let eta = match it {
                    0 => String::from("?"),
                    _ => ftime((elapsed / pct * (1. - pct)) as usize),
                };

                let l_bar = format!("{}{:>3}%|", desc, (100.0 * pct) as usize);
                let r_bar = format!(
                    "| {}/{} [{}<{}, {:.02}it/s{}]",
                    it, total, time, eta, its, postfix
                );

                let limit = (width as usize).saturating_sub(l_bar.len() + r_bar.len());

                let style: Vec<_> = self.config.style.as_ref().chars().collect();

                let filled = style[0];
                let (background, in_progress) = style[1..].split_last().unwrap();

                let m = in_progress.len();
                let n = ((limit as f64 * pct) * m as f64) as usize;
                let n_filled = n / m;

                let mut bar = filled.to_string().repeat(n_filled);

                if n_filled < limit {
                    bar.push(in_progress[n % m]);
                }

                // Unicode width is not considered at the moment
                if n_filled + 1 < limit {
                    let n_padding = limit - n_filled - 1;
                    let padding = background.to_string().repeat(n_padding);

                    bar.push_str(&padding);
                }

                fmt.write_fmt(format_args!("{}{}{}", l_bar, bar, r_bar))
            }
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        let mut new_state = Self::new(self.total);
        new_state.config = self.config.clone();
        new_state
    }
}

impl Drop for State {
    /// Automatically close a progress bar when it's dropped.
    fn drop(&mut self) {
        drop(self.close());
    }
}

/// Config decides how a progress bar is displayed
#[derive(Debug, Clone)]
struct Config {
    style: Style,
    width: Option<u16>,
    desc: Option<String>,
    postfix: Option<String>,
    interval: f64,
}

impl Config {
    /// Create a new progress bar config.
    fn new() -> Self {
        Self {
            style: Style::default(),
            desc: None,
            width: None,
            postfix: None,
            interval: 1.0 / 15.0,
        }
    }
}

/// Wrapping state in arc and mutex
type AtomicState = Arc<Mutex<State>>;
/// Identifier of a progress bar
type ID = u64;
/// Position of a progress bar
type Pos = u16;

/// Next unused ID
static NEXTID: AtomicU64 = AtomicU64::new(0);
/// How many rows are progress bars allowed to use. If unspecified,
/// use the terminal height.
static NROWS: AtomicU16 = AtomicU16::new(0);
/// Book-keeping the positions of all bars.
static POSITIONS: OnceLock<Mutex<HashMap<ID, Pos>>> = OnceLock::new();

/// Set how many on-going progress bar can be shown on the screen.
///
/// If specified, hides bars outside this limit. If unspecified, adjusts to
/// the terminal height.
pub fn set_max_progress_bars(nbars: u16) {
    let nrows = max(nbars + 1, 2);
    NROWS.swap(nrows, Ordering::Relaxed);
}

/// Warps the global [`POSITIONS`]
fn positions() -> &'static Mutex<HashMap<ID, Pos>> {
    POSITIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// If all progress bars are closed
fn is_finished() -> bool {
    positions().lock().unwrap().is_empty()
}

/// Retrieve the environment width and height
fn terminal_size() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((80, 64))
}

/// How many rows can progress bars take up. If unspecified,
/// uses the terminal height.
fn nrows() -> u16 {
    let nrows = NROWS.load(Ordering::Relaxed);

    if nrows != 0 {
        min(nrows, terminal_size().1)
    } else {
        terminal_size().1
    }
}

/// Find the next free position
fn next_free_pos() -> ID {
    let mut positions = positions().lock().unwrap();
    let next_id = NEXTID.fetch_add(1, Ordering::Relaxed);
    let next_pos = positions.values().max().map(|n| n + 1).unwrap_or(0);
    positions.insert(next_id, next_pos);

    next_id
}

/// When a bar is closed, rearrange the position of other progress bars.
fn reposition(id: ID) {
    let mut positions = positions().lock().unwrap();

    let closed_pos = *positions.get(&id).unwrap();

    positions.remove(&id);

    // Move upwards all the bars below the closed bar
    positions.iter_mut().for_each(|(_, pos)| {
        if *pos > closed_pos {
            *pos -= 1;
        }
    });
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::set_max_progress_bars;
    use crate::{style::Style, AvanceBar};

    fn progress_bar_ref(pb: &AvanceBar, n: u64, interval: u64) {
        for _ in 0..n {
            pb.update(1);

            thread::sleep(Duration::from_millis(interval));
        }
    }

    fn progress_bar(n: u64, interval: u64) {
        let pb = AvanceBar::new(n);
        progress_bar_ref(&pb, n, interval);
    }

    #[test]
    fn basic_bar() {
        progress_bar(100, 5);
    }

    #[test]
    fn bar_with_width() {
        let pb = AvanceBar::new(100);
        pb.set_width(60);

        progress_bar_ref(&pb, 100, 5);
    }

    #[test]
    fn reuse() {
        let pb1 = AvanceBar::new(100)
            .with_style(Style::Balloon)
            .with_width(90);
        progress_bar_ref(&pb1, 100, 5);
        pb1.close();

        let pb2 = AvanceBar::with_config_of(&pb1).with_total(200);
        progress_bar_ref(&pb2, 200, 5);
    }

    #[test]
    fn method_chain() {
        let pb = AvanceBar::new(100)
            .with_style(Style::Block)
            .with_width(90)
            .with_desc("task1");

        progress_bar_ref(&pb, 100, 5);
    }

    #[test]
    fn misc() {
        let pb = AvanceBar::new(100);
        pb.set_desc("avance");
        pb.set_style(Style::Balloon);
        pb.set_width(60);

        progress_bar_ref(&pb, 100, 10);
    }

    #[test]
    fn single_bar_multi_threads() {
        let pb = AvanceBar::new(300);

        std::thread::scope(|t| {
            t.spawn(|| progress_bar_ref(&pb, 100, 15));
            t.spawn(|| progress_bar_ref(&pb, 100, 10));
            t.spawn(|| progress_bar_ref(&pb, 100, 5));
        });
    }

    #[test]
    fn multiple_bars() {
        std::thread::scope(|t| {
            t.spawn(|| progress_bar(150, 5));
            t.spawn(|| progress_bar(300, 5));
            t.spawn(|| progress_bar(500, 5));
        });
    }

    #[test]
    fn overflowing() {
        set_max_progress_bars(3);

        let threads: Vec<_> = (0..15)
            .map(|i| thread::spawn(move || progress_bar(100 + 100 * (i % 5), 10 - i % 5)))
            .collect();

        for t in threads {
            t.join().unwrap();
        }
    }
}
