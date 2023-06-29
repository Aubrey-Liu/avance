//! A progress bar and all utilities.

use crossterm::cursor::{MoveDown, MoveToColumn, MoveUp};
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::tty::IsTty;
use crossterm::QueueableCommand;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Result, Write};
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Instant;

use crate::style::Style;

/// The progress bar
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

    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(size_hint.map(|s| s as u64)))),
        }
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

        drop(state.draw(None));
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

        drop(state.clear());
        drop(state.draw(None));
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

        drop(state.draw(None));
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

    /// Manually stop the progress bar. Usually users don't have to call this
    /// method directly, as a progress bar will close automatically when dropped.
    pub fn close(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        reposition(state.id);

        state.force_update();
        state.draw(Some(0))?;

        let mut target = std::io::stderr().lock();
        if target.is_tty() {
            writeln!(target)
        } else {
            Ok(())
        }
    }

    /// Refresh the progress bar.
    ///
    /// # Panics
    /// This method would only panic when another thread was using a progress bar and panicked.
    fn refresh(&self) {
        let state = self.state.lock().unwrap();

        drop(state.draw(None))
    }
}

impl Drop for AvanceBar {
    /// Automatically close a progress bar when it's dropped.
    fn drop(&mut self) {
        drop(self.close());
    }
}

struct State {
    config: Config,
    begin: Instant,
    last: Instant,
    interval: f64,
    id: PosID,
    n: u64,
    cached: u64,
    total: Option<u64>,
}

impl State {
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

    fn update(&mut self, n: u64) {
        self.cached += n;

        if matches!(self.total, Some(total) if self.n + self.cached >= total) {
            self.force_update();
            drop(self.draw(None));
        } else if self.last.elapsed().as_secs_f64() >= self.interval {
            self.n += self.cached;
            self.cached = 0;
            self.last = Instant::now();
            drop(self.draw(None));
        }
    }

    fn force_update(&mut self) {
        self.n = if let Some(total) = self.total {
            min(total, self.n + self.cached)
        } else {
            self.n + self.cached
        };
        self.cached = 0;
        self.last = Instant::now();
    }

    fn draw(&self, pos: Option<u16>) -> Result<()> {
        let mut target = std::io::stderr().lock();
        if !target.is_tty() {
            return Ok(());
        }

        let pos = if let Some(pos) = pos {
            pos
        } else {
            self.get_pos()
        };

        let nrows = NROWS.load(Ordering::Relaxed);
        let (ncols, nrows) =
            crossterm::terminal::size().map_or((80, nrows), |(c, r)| (c, min(r, nrows)));
        if pos >= nrows {
            return Ok(());
        }

        let msg = if pos == nrows - 1 {
            "... (more hidden) ...".to_string()
        } else {
            format!("{}", self)
        };
        let msg = format!("{:1$}", msg, ncols as usize);

        // target.queue(Hide)?;
        if pos != 0 {
            target.queue(MoveDown(pos))?;
            target.queue(MoveToColumn(0))?;
            target.queue(Print(msg))?;
            target.queue(MoveUp(pos))?;
            target.queue(MoveToColumn(ncols))?;
        } else {
            target.queue(MoveToColumn(0))?;
            target.queue(Print(msg))?;
        }
        // target.queue(Show)?;
        target.flush()
    }

    fn clear(&self) -> Result<()> {
        let mut target = std::io::stderr().lock();
        if !target.is_tty() {
            return Ok(());
        }

        let pos = self.get_pos();

        let nrows = NROWS.load(Ordering::Relaxed);
        let nrows = crossterm::terminal::size().map_or(nrows, |(_, r)| min(r, nrows));
        if pos >= nrows {
            return Ok(());
        }

        if pos != 0 {
            target.queue(MoveDown(pos))?;
            target.queue(Clear(ClearType::CurrentLine))?;
            target.queue(MoveUp(pos))?;
        } else {
            target.queue(Clear(ClearType::CurrentLine))?;
        }
        target.flush()
    }

    fn get_pos(&self) -> Pos {
        let positions = positions().lock().unwrap();
        *positions.get(&self.id).unwrap()
    }
}

impl Display for State {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        let elapsed = self.begin.elapsed().as_secs_f64();

        let Config { desc, width, style } = &self.config;
        let desc = desc.clone().map_or(String::new(), |desc| desc + ": ");
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
                let tqdm = {
                    let limit = (width as usize).saturating_sub(bra_.len() + _ket.len());
                    let pattern: Vec<_> = style.as_ref().chars().collect();

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

/// Config decides how a progress bar is displayed
pub struct Config {
    style: Style,
    width: Option<u16>,
    desc: Option<String>,
}

impl Config {
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
type PosID = u64;
type Pos = u16;

static NEXTID: AtomicU64 = AtomicU64::new(0);
static NROWS: AtomicU16 = AtomicU16::new(20);
static POSITIONS: OnceLock<Mutex<HashMap<PosID, Pos>>> = OnceLock::new();

/// This method decides how many progress bar can be shown on the screen.
/// If specified, hides bars outside this limit. If unspecified, adjusts to
/// environment height.
pub fn set_max_progress_bars(nbars: u16) {
    let nrows = max(nbars + 1, 2);
    NROWS.swap(nrows, Ordering::Relaxed);
}

fn positions() -> &'static Mutex<HashMap<PosID, Pos>> {
    POSITIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_free_pos() -> PosID {
    let mut positions = positions().lock().unwrap();
    let next_id = NEXTID.fetch_add(1, Ordering::Relaxed);
    let next_pos = positions.values().max().map(|n| n + 1).unwrap_or(0);
    positions.insert(next_id, next_pos);

    next_id
}

fn reposition(id: PosID) {
    let mut positions = positions().lock().unwrap();

    let closed_pos = *positions.get(&id).unwrap();
    if closed_pos >= NROWS.load(Ordering::Relaxed) - 1 {
        positions.remove(&id);
        return;
    }

    if let Some((&chosen_id, _)) = positions
        .iter()
        .find(|(_, &pos)| pos >= NROWS.load(Ordering::Relaxed) - 1)
    {
        // Move an overflowed bar up to fill the blank
        positions.remove(&id);
        *positions.get_mut(&chosen_id).unwrap() = closed_pos;
    } else {
        // If we can't find an overflowed bar, move all bars upwards when bar.pos > pos
        positions.remove(&id);
        positions.iter_mut().for_each(|(_, pos)| {
            if *pos > closed_pos {
                *pos -= 1;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::atomic::Ordering, thread, time::Duration};

    use super::NROWS;
    use crate::{style::Style, AvanceBar};

    fn progress_bar_ref(pb: &AvanceBar, n: u64, interval: u64) {
        for _ in 0..n {
            pb.update(1);

            std::thread::sleep(Duration::from_millis(interval));
        }
    }

    fn progress_bar(n: u64, interval: u64) {
        let pb = AvanceBar::new(n);
        progress_bar_ref(&pb, n, interval);
    }

    #[test]
    fn basic_bar() {
        progress_bar(100, 20);
    }

    #[test]
    fn bar_with_width() {
        let pb = AvanceBar::new(100);
        pb.set_width(60);

        progress_bar_ref(&pb, 100, 20);
    }

    #[test]
    fn misc() {
        let pb = AvanceBar::new(100);
        pb.set_description("avance");
        pb.set_style(Style::Balloon);
        pb.set_width(60);

        progress_bar_ref(&pb, 100, 20);
    }

    #[test]
    fn single_bar_multi_threads() {
        let pb = AvanceBar::new(300);

        std::thread::scope(|t| {
            t.spawn(|| progress_bar_ref(&pb, 100, 30));
            t.spawn(|| progress_bar_ref(&pb, 100, 20));
            t.spawn(|| progress_bar_ref(&pb, 100, 10));
        });
    }

    #[test]
    fn multiple_bars() {
        std::thread::scope(|t| {
            t.spawn(|| progress_bar(150, 10));
            t.spawn(|| progress_bar(300, 10));
            t.spawn(|| progress_bar(500, 10));
        });
    }

    #[test]
    fn overflowing() {
        NROWS.swap(10, Ordering::Relaxed);

        let threads: Vec<_> = (0..15)
            .map(|i| thread::spawn(move || progress_bar(80 * (i % 4 + 1), 10 * (i % 4 + 1))))
            .collect();

        for t in threads {
            t.join().unwrap();
        }
    }
}
