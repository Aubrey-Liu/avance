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

use crate::style::Style;

/// The progress bar
#[derive(Debug)]
pub struct AvanceBar {
    state: AtomicState,
}

impl AvanceBar {
    /// Create a new progress bar
    ///
    /// # Examples
    /// ```
    /// use avance::AvanceBar;
    /// let pb = AvanceBar::new(1000);
    /// ```
    pub fn new(total: u64) -> Self {
        let pb = AvanceBar {
            state: Arc::new(Mutex::new(State::new(Some(total)))),
        };
        pb.refresh();
        pb
    }

    /// Create a progress bar from an iterator's size hint
    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(size_hint.map(|s| s as u64)))),
        }
    }

    /// Advance the progress bar by n steps
    ///     /// Advance the progress bar by one step, equal to `update(1)`
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::AvanceBar;
    /// # use std::cmp::min;
    /// # use std::thread;
    /// # use std::time::Duration;
    ///
    /// let n_bytes = 1024 * 1024;
    /// let mut bytes_read = 0;
    ///
    /// let pb = AvanceBar::new(n_bytes);
    /// pb.set_style(avance::Style::Block);
    /// pb.set_description("reading");
    ///
    /// while bytes_read < n_bytes {
    ///     bytes_read = min(bytes_read + 1378, n_bytes);
    ///     pb.update(1378);
    /// }
    ///
    /// pb.set_description("done");
    /// ```
    pub fn update(&self, n: u64) {
        let mut state = self.state.lock().unwrap();

        state.update(n);
    }

    /// Advance the progress bar by one step, equal to `update(1)`
    ///
    /// # Examples
    /// ```
    /// use avance::bar::AvanceBar;
    ///
    /// let pb = AvanceBar::new(1000);
    /// for _ in 0..1000 {
    ///     pb.inc();
    ///     // do something here
    /// }
    /// ```
    pub fn inc(&self) {
        self.update(1);
    }

    /// Set the description of a progress bar.
    ///
    /// For example, if you set "avance" as the description,
    /// the progress bar will be like:
    ///
    /// `avance: 100%|*************| 100/100 [00:02<00:00, 44.18it/s]`
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::AvanceBar;
    ///
    /// let pb = AvanceBar::new(1000);
    /// pb.set_description("task name");
    /// for _ in 0..1000 {
    ///    // ...
    ///    pb.inc();
    /// }
    /// ```
    pub fn set_description(&self, desc: impl ToString) {
        let mut state = self.state.lock().unwrap();
        state.config.desc = Some(desc.to_string());
        let _ = state.draw(None);
    }

    /// Set a progress bar's width
    ///
    /// If width is larger than terminal width, progress bar will adjust
    /// to the terminal width.
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::AvanceBar;
    ///
    /// let pb = AvanceBar::new(1000);
    /// pb.set_width(80);
    /// for _ in 0..1000 {
    ///    // ...
    ///    pb.inc();
    /// }
    /// ```
    pub fn set_width(&self, width: u16) {
        let mut state = self.state.lock().unwrap();
        state.config.width = Some(width);
        let _ = state.clear();
        let _ = state.draw(None);
    }

    /// Set the style of a progress bar.
    ///
    /// See available styles in [`Style`]
    ///
    /// # Examples
    ///
    /// ```
    /// use avance::{AvanceBar, Style};
    ///
    /// let pb = AvanceBar::new(1000);
    /// pb.set_style(Style::Block);
    /// for _ in 0..1000 {
    ///    // ...
    ///    pb.inc();
    /// }
    /// ```
    pub fn set_style(&self, style: Style) {
        let mut state = self.state.lock().unwrap();
        state.config.style = style;
        let _ = state.draw(None);
    }

    /// Builder-like function for a progress bar with description
    pub fn with_desc(self, desc: impl ToString) -> Self {
        self.set_description(desc);
        self
    }

    /// Builder-like function for a progress bar with width
    pub fn with_width(self, width: u16) -> Self {
        self.set_width(width);
        self
    }

    /// Builder-like function for a progress bar with style
    pub fn with_style(self, style: Style) -> Self {
        self.set_style(style);
        self
    }

    /// Manually stop the progress bar. Usually users don't have to call this
    /// method directly, as a progress bar will close automatically when dropped.
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        if !state.drawable() {
            // already closed
            return;
        }

        // Don't update when there's nothing new
        if !matches!(state.total, Some(t) if t == state.n) {
            state.force_update();
        }
        let _ = state.draw(Some(0));

        let mut target = std::io::stderr().lock();
        if target.is_tty() {
            let _ = writeln!(target);
        }

        reposition(state.id);
    }

    /// Refresh the progress bar.
    ///
    /// # Panics
    /// This method will panic when another thread was using a progress bar and panicked.
    fn refresh(&self) {
        let state = self.state.lock().unwrap();
        let _ = state.draw(None);
    }
}

impl Clone for AvanceBar {
    /// For convenient reusing of a progress bar
    ///
    /// # Examples
    /// ```     
    /// use avance::{AvanceBar, Style};  
    ///
    /// let pb = AvanceBar::new(100)
    ///     .with_style(Style::Balloon)
    ///     .with_width(90)
    ///     .with_desc("task1");
    ///  
    /// for _ in 0..100 {
    ///     // ...
    ///     pb.inc();
    /// }
    ///
    /// // reuse the style and width of pb
    /// let pb = pb.clone().with_desc("task2");
    /// for _ in 0..100 {
    ///     // ...
    ///     pb.inc();
    /// }
    /// ```
    fn clone(&self) -> Self {
        let old_state = self.state.lock().unwrap();
        let mut state = State::new(old_state.total);
        state.config = old_state.config.clone();

        let pb = Self {
            state: Arc::new(Mutex::new(state)),
        };
        pb.refresh();
        pb
    }
}

impl Drop for AvanceBar {
    /// Automatically close a progress bar when it's dropped.
    fn drop(&mut self) {
        self.close();
    }
}

/// The inner state of a progress bar
#[derive(Debug, Clone)]
struct State {
    config: Config,
    begin: Instant,
    last: Instant,
    interval: f64,
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
            interval: 1.0 / 15.0,
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

        if self.last.elapsed().as_secs_f64() >= self.interval {
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
            target.queue(Print("\n".repeat(pos as usize)))?;
            target.queue(Print(msg))?;
            target.queue(MoveUp(pos))?;
            target.queue(MoveToColumn(ncols))?;
        } else {
            target.queue(MoveToColumn(0))?;
            target.queue(Print(msg))?;
        }
        target.flush()
    }

    /// Is a progress bar able to be displayed
    fn drawable(&self) -> bool {
        // is_terminal is stable on 1.70.0
        stderr().is_tty() && self.try_get_pos().is_some()
    }

    /// Sweep a progress bar from the terminal
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
            target.queue(Print("\n".repeat(pos as usize)))?;
            target.queue(Clear(ClearType::CurrentLine))?;
            target.queue(MoveUp(pos))?;
        } else {
            target.queue(Clear(ClearType::CurrentLine))?;
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

        let Config { desc, width, style } = &self.config;
        let desc = desc.clone().map_or_else(String::new, |desc| desc + ": ");
        let terminal_width = terminal::size().map_or(80, |(c, _)| c);
        let width = width.map_or(terminal_width, |w| min(w, terminal_width));

        /// Time formatting function, which omits the leading 0s
        fn ftime(seconds: usize) -> String {
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            match seconds / 3600 {
                0 => format!("{:02}:{:02}", m, s),
                h => format!("{:02}:{:02}:{:02}", h, m, s),
            }
        }

        let it = self.n;
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

                let limit = (width as usize).saturating_sub(bra_.len() + _ket.len());

                let style: Vec<_> = style.as_ref().chars().collect();
                let background = style[0];
                let pattern = &style[1..];

                let m = pattern.len();
                let n = ((limit as f64 * pct) * m as f64) as usize;
                let filled = n / m;

                let mut pb = pattern.last().unwrap().to_string().repeat(filled);

                if filled < limit {
                    pb.push(pattern[n % m]);
                }

                let filled = filled + 1;
                if filled < limit {
                    let padding = background.to_string().repeat(limit - filled);

                    pb.push_str(&padding);
                }

                fmt.write_fmt(format_args!("{}{}{}", bra_, pb, _ket))
            }
        }
    }
}

/// Config decides how a progress bar is displayed
#[derive(Debug, Clone)]
struct Config {
    style: Style,
    width: Option<u16>,
    desc: Option<String>,
}

impl Config {
    /// Create a new progress bar config.
    fn new() -> Self {
        Self {
            style: Style::default(),
            desc: None,
            width: None,
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
/// How many rows are this lib allowed to use. If unspecified,
/// use the terminal height.
static NROWS: AtomicU16 = AtomicU16::new(0);
/// Book-keeping all progress bars' positions.
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
        let pb = AvanceBar::new(100);
        pb.set_style(Style::Balloon);
        pb.set_width(90);

        progress_bar_ref(&pb, 100, 5);

        // style and width can be reused
        let pb2 = pb.clone();
        progress_bar_ref(&pb2, 100, 5);
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
        pb.set_description("avance");
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
