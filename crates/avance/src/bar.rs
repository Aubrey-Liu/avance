//! TODO: documentation

use crossterm::{
    cursor::MoveUp,
    style::Print,
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use std::{
    cmp::min,
    collections::HashMap,
    fmt::{Display, Formatter},
    io::{Result, Write},
    sync::{
        atomic::{AtomicU16, AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Instant,
};

use crate::style::Style;

pub struct AvanceBar {
    state: AtomicState,
}

impl AvanceBar {
    pub fn new(total: u64) -> Self {
        let bar = AvanceBar {
            state: Arc::new(Mutex::new(State::new(Some(total)))),
        };
        bar.refresh();
        bar
    }

    pub(crate) fn with_hint(size_hint: Option<usize>) -> Self {
        AvanceBar {
            state: Arc::new(Mutex::new(State::new(size_hint.map(|s| s as u64)))),
        }
    }

    pub fn set_description(&self, desc: impl ToString) {
        {
            let mut state = self.state.lock().unwrap();
            state.config.desc = Some(desc.to_string());
        }
        self.refresh();
    }

    pub fn set_width(&self, width: u16) {
        {
            let mut state = self.state.lock().unwrap();
            state.config.width = Some(width);
        }
        self.refresh();
    }

    pub fn set_style(&self, style: Style) {
        {
            let mut state = self.state.lock().unwrap();
            state.config.style = style;
        }
        self.refresh();
    }

    /// Refresh the progress bar instantly
    pub fn refresh(&self) {
        let state = self.state.lock().unwrap();

        drop(state.draw(None))
    }

    pub fn update(&self, n: u64) {
        let mut state = self.state.lock().unwrap();

        state.update(n);
    }

    pub fn close(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        reposition(state.id);

        state.force_update();
        state.draw(Some(0))?;

        let mut target = std::io::stderr().lock();
        writeln!(target)
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

        let pos = if let Some(pos) = pos {
            pos
        } else {
            self.get_pos()
        };

        let nrows = NROWS.load(Ordering::Relaxed);
        if pos >= nrows {
            return Ok(());
        }

        if pos != 0 {
            target.queue(Print("\n".repeat(pos as usize)))?;
            target.queue(Clear(ClearType::CurrentLine))?;
            if pos == nrows - 1 {
                target.queue(Print("... (more hidden) ..."))?;
            } else {
                target.queue(Print(self))?;
            }
            target.queue(MoveUp(pos))?;
        } else {
            target.queue(Clear(ClearType::CurrentLine))?;
            target.write_fmt(format_args!("\r{}", self))?;
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

type AtomicState = Arc<Mutex<State>>;
type PosID = u64;
type Pos = u16;

static NEXTID: AtomicU64 = AtomicU64::new(0);
static NROWS: AtomicU16 = AtomicU16::new(20);
static POSITIONS: OnceLock<Mutex<HashMap<PosID, Pos>>> = OnceLock::new();

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
        positions.remove(&id);
        *positions.get_mut(&chosen_id).unwrap() = closed_pos;
    } else {
        // If we can't find an overflowed bar, move all bars upwards when bar.pos > pos
        positions.remove(&id);
        positions.iter_mut().for_each(|(_, pos)| {
            if *pos > closed_pos {
                *pos = *pos - 1;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bar::{next_free_pos, positions},
        style::Style,
        AvanceBar,
    };
    use std::{sync::atomic::Ordering, thread, time::Duration};

    use super::NROWS;

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
    fn bar_with_width() {
        let bar = AvanceBar::new(100);
        bar.set_width(60);

        for _ in 0..100 {
            bar.update(1);

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn misc() {
        let bar = AvanceBar::new(100);
        bar.set_description("avance");
        bar.set_style(Style::Balloon);
        bar.set_width(76);

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
                for _ in 0..50 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(100));
                }
            });
            t.spawn(|| {
                for _ in 0..100 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(50));
                }
            });
            t.spawn(|| {
                for _ in 0..150 {
                    bar.update(1);

                    std::thread::sleep(Duration::from_millis(10));
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

    #[test]
    fn overflowing() {
        NROWS.swap(5, Ordering::Relaxed);

        let threads: Vec<_> = (0..8)
            .map(|i| {
                thread::spawn(move || {
                    let n = 80 * (i % 4 + 1);
                    let bar = AvanceBar::new(n);
                    for _ in 0..n {
                        bar.update(1);

                        std::thread::sleep(Duration::from_millis(15 * (i % 4 + 1)));
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }
    }
}
