//! A progress bar and all utilities.

use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::tty::IsTty;
use crossterm::QueueableCommand;
use std::borrow::Cow;
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
    progress: Arc<AtomicProgress>,
}

// Public Interface
impl AvanceBar {
    /// Create a new progress bar
    pub fn new(total: u64) -> Self {
        let progress = Arc::new(AtomicProgress::new());
        let pb = AvanceBar {
            state: Arc::new(Mutex::new(State::new(Some(total), Arc::clone(&progress)))),
            progress,
        };
        pb.refresh();
        pb
    }

    /// Build a new progress bar from the config of another progress bar.
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
        let old_state = pb.state.lock().unwrap();
        let progress = Arc::new(AtomicProgress::new());
        let mut new_state = State::new(old_state.total, Arc::clone(&progress));
        new_state.config = old_state.config.clone();
        let new_pb = AvanceBar {
            state: Arc::new(Mutex::new(new_state)),
            progress,
        };
        new_pb.refresh();
        new_pb
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
    pub fn with_iter<Iter: Iterator>(&self, iter: Iter) -> AvanceIter<Iter> {
        AvanceIter {
            iter,
            bar: self.clone(),
        }
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
    pub fn with_style_str(self, s: &'static str) -> Self {
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
    /// let pb = AvanceBar::new(1000).with_desc("task name");
    /// ```
    pub fn with_desc(self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.set_desc(desc);
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

    /// Builder-like function for displaying human readable numbers in a progress bar.
    ///
    /// If unit_scale (default: false) is set true, prints the number of iterations
    /// with an appropriate SI metric prefix (k = 10^3, M = 10^6, etc.)
    pub fn with_unit_scale(self, unit_scale: bool) -> Self {
        self.set_unit_scale(unit_scale);
        self
    }

    /// Override the postfix of a progress bar.
    ///
    /// Postfix is usually used for **dynamically** displaying some
    /// additional information, such as the accuracy when training a model.
    ///
    /// See [`AvanceIter::with_pb`] if you want to change the postfix when
    /// progressing with an iterator.
    pub fn set_postfix(&self, postfix: impl Into<Cow<'static, str>>) {
        let mut state = self.state.lock().unwrap();
        state.config.postfix = Some(postfix.into());
        let _ = state.draw_to_stderr(None);
    }

    /// Advance the progress bar by n steps.
    pub fn update(&self, n: u64) {
        self.progress.inc(n);

        if self.progress.ready() {
            let _ = self.state.lock().unwrap().draw_to_stderr(None);
            self.progress.update();
        }
    }

    /// Advance the progress bar by one step, with the same effect as
    /// [`update(1)`](Self::update). If you don't want to invoke inc
    /// manually, see another method at [`with_iter`](Self::with_iter).
    ///
    /// # Examples
    /// ```
    /// # use avance::AvanceBar;
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
        let _ = state.draw_to_stderr(None);
    }

    /// Set the user-custom style of a progress bar.
    pub fn set_style_str(&self, s: impl Into<Cow<'static, str>>) {
        let mut state = self.state.lock().unwrap();
        state.config.style = Style::Custom(s.into());
        let _ = state.draw_to_stderr(None);
    }

    /// Set a progress bar's width
    pub fn set_width(&self, width: u16) {
        let mut state = self.state.lock().unwrap();
        state.config.width = Some(width);
        let _ = state.clear();
        let _ = state.draw_to_stderr(None);
    }

    /// Set the description (prefix) of a progress bar.
    pub fn set_desc(&self, desc: impl Into<Cow<'static, str>>) {
        let mut state = self.state.lock().unwrap();
        state.config.desc = Some(desc.into());
        let _ = state.draw_to_stderr(None);
    }

    /// Set the length of a progress bar.
    pub fn set_total(&self, total: u64) {
        let mut state = self.state.lock().unwrap();
        state.total = Some(total);
        let _ = state.draw_to_stderr(None);
    }

    /// If unit_scale (default: false) is set true, prints the number of iterations
    /// with an appropriate SI metric prefix.
    pub fn set_unit_scale(&self, unit_scale: bool) {
        self.state.lock().unwrap().config.unit_scale = unit_scale;
    }
}

// Private Interface
impl AvanceBar {
    /// Creates a progress bar from an iterator's size hint
    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        let progress = Arc::new(AtomicProgress::new());
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(
                size_hint.map(|s| s as u64),
                Arc::clone(&progress),
            ))),
            progress,
        }
    }

    /// Refresh the progress bar.
    fn refresh(&self) {
        let state = self.state.lock().unwrap();
        let _ = state.draw_to_stderr(None);
    }
}

#[derive(Debug)]
struct State {
    id: ID,
    config: Config,
    total: Option<u64>,
    progress: Arc<AtomicProgress>,
}

impl State {
    fn new(total: Option<u64>, progress: Arc<AtomicProgress>) -> Self {
        Self {
            id: next_free_pos(),
            config: Config::new(),
            total,
            progress,
        }
    }

    fn draw<W: Write>(&self, pos: Option<u16>, target: &mut W) -> Result<()> {
        if pos.is_none() && !self.drawable() {
            return Ok(());
        }
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

    fn draw_to_stderr(&self, pos: Option<u16>) -> Result<()> {
        self.draw(pos, &mut stderr().lock())
    }

    fn drawable(&self) -> bool {
        // is_terminal is stable on 1.70.0
        stderr().is_tty() && self.try_get_pos().is_some()
    }

    fn close(&mut self) -> Result<()> {
        if !self.drawable() {
            // already closed
            return Ok(());
        }

        // Close the current bar and move up other bars
        reposition(self.id);

        let mut target = stderr().lock();

        // force update (only displaying average its)
        self.progress.update();
        let _ = self.draw(Some(0), &mut target);

        // Move cursor to the end of the next line
        let ncols = terminal_size().0;

        target.queue(Print('\n'))?;
        if !is_finished() {
            // only do this when some bars are still in progress
            target.queue(MoveToColumn(ncols))?;
        }
        target.flush()
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

    fn try_get_pos(&self) -> Option<Pos> {
        let positions = positions().lock().unwrap();
        positions.get(&self.id).copied()
    }

    fn get_pos(&self) -> Pos {
        self.try_get_pos().unwrap()
    }
}

impl Display for State {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        use format::*;

        let elapsed = self.progress.begin.elapsed().as_secs_f64();
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

        let n = self.progress.n.load(Ordering::Relaxed);
        let last_n = self.progress.last.load(Ordering::Relaxed);
        let since_last = self.progress.since_last() as f64 / 1e9;

        // smoothing
        let factor = 0.7;
        let its = match n - last_n {
            0 => n as f64 / elapsed,
            gap => (n as f64 / elapsed) * factor + (gap as f64 / since_last) * (1.0 - factor),
        };

        let time = format_time(elapsed as u64);

        match self.total {
            None => fmt.write_fmt(format_args!(
                "{}{}it [{}, {:.02}it/s]{}",
                desc, n, time, its, postfix
            )),

            Some(total) => {
                let pct = (n as f64 / total as f64).clamp(0.0, 1.0);
                let eta = match n {
                    0 => String::from("?"),
                    _ => format_time((elapsed / pct * (1. - pct)) as u64),
                };

                let l_bar = format!("{}{:>3}%|", desc, (100.0 * pct) as u64);
                let r_bar = match self.config.unit_scale {
                    true => format!(
                        "| {}/{} [{}<{}, {:.02}it/s{}]",
                        format_sizeof(n),
                        format_sizeof(total),
                        time,
                        eta,
                        its,
                        postfix
                    ),
                    false => format!(
                        "| {}/{} [{}<{}, {:.02}it/s{}]",
                        n, total, time, eta, its, postfix
                    ),
                };
                let limit = (width as usize).saturating_sub(l_bar.len() + r_bar.len());

                let style: Vec<_> = self.config.style.as_ref().chars().collect();

                let filled = style[0];
                let (background, in_progress) = style[1..].split_last().unwrap();

                let m = in_progress.len();
                let k = ((limit as f64 * pct) * m as f64) as usize;
                let n_filled = k / m;
                let current = k % m;

                let mut bar = filled.to_string().repeat(n_filled);

                if n_filled < limit {
                    bar.push(in_progress[current]);
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

impl Drop for State {
    fn drop(&mut self) {
        drop(self.close());
    }
}

#[derive(Debug)]
struct AtomicProgress {
    begin: Instant,
    prev: AtomicU64,
    last: AtomicU64,
    n: AtomicU64,
}

impl AtomicProgress {
    fn new() -> Self {
        Self {
            begin: Instant::now(),
            prev: AtomicU64::new(0),
            last: AtomicU64::new(0),
            n: AtomicU64::new(0),
        }
    }

    fn inc(&self, delta: u64) {
        self.n.fetch_add(delta, Ordering::AcqRel);
    }

    fn ready(&self) -> bool {
        self.since_last() > INTERVAL
    }

    fn update(&self) {
        self.prev
            .store(self.begin.elapsed().as_nanos() as u64, Ordering::Release);
        self.last
            .store(self.n.load(Ordering::Acquire), Ordering::Release);
    }

    fn since_last(&self) -> u64 {
        let prev = self.prev.load(Ordering::Acquire);
        let since_begin = self.begin.elapsed().as_nanos() as u64;
        since_begin.saturating_sub(prev)
    }
}

#[derive(Debug, Clone)]
struct Config {
    style: Style,
    width: Option<u16>,
    desc: Option<Cow<'static, str>>,
    postfix: Option<Cow<'static, str>>,
    unit_scale: bool,
}

impl Config {
    fn new() -> Self {
        Self {
            style: Style::default(),
            desc: None,
            width: None,
            postfix: None,
            unit_scale: false,
        }
    }
}

type AtomicState = Arc<Mutex<State>>;
type ID = u64;
type Pos = u16;

/// Minimun update interval (in nanoseconds)
const INTERVAL: u64 = 100_000_000;

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

fn positions() -> &'static Mutex<HashMap<ID, Pos>> {
    POSITIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_finished() -> bool {
    positions().lock().unwrap().is_empty()
}

fn terminal_size() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((80, 64))
}

fn nrows() -> u16 {
    let nrows = NROWS.load(Ordering::Relaxed);

    if nrows != 0 {
        min(nrows, terminal_size().1)
    } else {
        terminal_size().1
    }
}

fn next_free_pos() -> ID {
    let mut positions = positions().lock().unwrap();
    let next_id = NEXTID.fetch_add(1, Ordering::Relaxed);
    let next_pos = positions.values().max().map(|n| n + 1).unwrap_or(0);
    positions.insert(next_id, next_pos);

    next_id
}

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
    use std::time::Instant;

    use crate::AvanceBar;

    #[test]
    fn performance() {
        let n = 20_000_000;

        let start = Instant::now();
        for _ in 0..n {}
        let du = Instant::now().duration_since(start).as_secs_f64();
        println!("raw: {:.2} it/s", n as f64 / du);

        let pb = AvanceBar::new(n);
        for _ in pb.with_iter(0..n) {}
    }
}
