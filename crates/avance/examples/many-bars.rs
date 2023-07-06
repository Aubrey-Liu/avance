use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    set_max_progress_bars(3);

    std::thread::scope(|t| {
        for i in 0..15 {
            t.spawn(move || {
                let pb = AvanceBar::new(1200);
                pb.set_desc(format!("task{}", i));

                for _ in 0..1200 {
                    thread::sleep(Duration::from_millis(3 + i % 5));
                    pb.inc();
                }
            });
        }
    });
}
