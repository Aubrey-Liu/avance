//! TODO: documentation

use crossterm::{cursor::MoveUp, style::Print, QueueableCommand};
use std::{
    cmp::min,
    collections::BTreeSet,
    fmt::{Display, Formatter},
    io::{Result, Write},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Instant,
};

use crate::style::Style;

pub struct AvanceBar {
    state: Arc<Mutex<State>>,
}

impl AvanceBar {
    pub fn new(total: u64) -> Self {
        let bar = AvanceBar {
            state: Arc::new(Mutex::new(State::new(Some(total)))),
        };

        drop(bar.refresh());

        bar
    }

    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(size_hint.map(|s| s as u64)))),
        }
    }

    /// Refresh the progress bar instantly
    pub fn refresh(&self) -> Result<()> {
        let state = self.state.lock().unwrap();

        state.draw(None)
    }

    pub fn update(&self, n: u64) {
        let mut state = self.state.lock().unwrap();

        state.update(n);
    }

    pub fn close(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        reposition(state.pos);
        state.force_update();
        state.draw(Some(0))?;

        let mut target = std::io::stderr().lock();
        write!(target, "\n")
    }
}

impl Drop for AvanceBar {
    fn drop(&mut self) {
        drop(self.close());
    }
}

struct State {
    config: Config,
    begin: Instant,
    last: Instant,
    refresh: f64,
    pos: u16,
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
            refresh: 1.0 / 15.0,
            pos: next_free_pos(),
            n: 0,
            cached: 0,
            total,
        }
    }

    fn update(&mut self, n: u64) {
        self.cached += n;

        if matches!(self.total, Some(total) if self.n + self.cached >= total) {
            self.force_update();
        } else if self.last.elapsed().as_secs_f64() >= self.refresh {
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

        let pos = if let Some(pos) = pos { pos } else { self.pos };

        if pos != 0 {
            target.queue(Print("\n".repeat(self.pos as usize)))?;
            target.queue(Print(self))?;
            target.queue(MoveUp(self.pos))?;
        } else {
            target.write_fmt(format_args!("\r{}", self))?;
        }

        target.flush()
    }
}

impl Display for State {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        let elapsed = self.begin.elapsed().as_secs_f64();

        let Config { desc, width, style } = &self.config;
        let desc = desc.clone().map_or(String::new(), |desc| desc + ": ");
        let width = width.unwrap_or_else(|| crossterm::terminal::size().unwrap().0);

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

static NROWS: AtomicU16 = AtomicU16::new(20);
static POSITIONS: OnceLock<Mutex<BTreeSet<u16>>> = OnceLock::new();

fn positions() -> &'static Mutex<BTreeSet<u16>> {
    POSITIONS.get_or_init(|| Mutex::new(BTreeSet::new()))
}

fn next_free_pos() -> u16 {
    let mut positions = positions().lock().unwrap();
    let mut next_pos = 0;
    positions.iter().find(|&&pos| {
        if pos == next_pos {
            next_pos += 1;
            false
        } else {
            true
        }
    });
    positions.insert(next_pos);

    next_pos
}

fn reposition(pos: u16) {
    let mut positions = positions().lock().unwrap();

    if pos >= NROWS.load(Ordering::Relaxed) - 1 {
        positions.remove(&pos);
        return;
    }

    if let Some(&overflowed_pos) = positions
        .iter()
        .find(|&&pos| pos >= NROWS.load(Ordering::Relaxed) - 1)
    {
        positions.remove(&overflowed_pos);
    } else {
        positions.remove(&pos);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        bar::{next_free_pos, positions},
        AvanceBar,
    };

    #[test]
    fn check_next_free_pos() {
        assert_eq!(next_free_pos(), 0);
        assert_eq!(next_free_pos(), 1);
        assert_eq!(next_free_pos(), 2);

        {
            let mut positions = positions().lock().unwrap();
            positions.remove(&1);
        }

        assert_eq!(next_free_pos(), 1);
    }

    #[test]
    fn basic_bar() {
        let bar = AvanceBar::new(100);
        for _ in 0..100 {
            bar.update(1);

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn single_bar_multi_threads() {
        let bar = AvanceBar::new(300);

        std::thread::scope(|t| {
            t.spawn(|| {
                for _ in 0..100 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(50));
                }
            });
            t.spawn(|| {
                for _ in 0..100 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(50));
                }
            });
            t.spawn(|| {
                for _ in 0..100 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(50));
                }
            });
        });
    }

    #[test]
    fn multiple_bars() {
        std::thread::scope(|t| {
            t.spawn(|| {
                let bar = AvanceBar::new(150);
                for _ in 0..150 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(30));
                }
            });
            t.spawn(|| {
                let bar = AvanceBar::new(300);
                for _ in 0..300 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(30));
                }
            });
            t.spawn(|| {
                let bar = AvanceBar::new(500);
                for _ in 0..500 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(30));
                }
            });
        });
    }
}
