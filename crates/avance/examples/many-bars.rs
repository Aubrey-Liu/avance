use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    set_max_progress_bars(3);

    std::thread::scope(|t| {
        for i in 0..15 {
            t.spawn(move || {
                AvanceBar::new(1200)
                    .with_desc(format!("task{}", i))
                    .with_iter(0..1200)
                    .for_each(|_| thread::sleep(Duration::from_millis(3 + i % 5)));
            });
        }
    });
}
