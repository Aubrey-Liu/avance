use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    std::thread::scope(|t| {
        t.spawn(|| {
            let pb = AvanceBar::new(1200).with_desc("task1");

            for _ in 0..1200 {
                thread::sleep(Duration::from_millis(3));
                pb.inc();
            }
        });
        t.spawn(|| {
            let pb = AvanceBar::new(1000).with_desc("task2");

            for _ in 0..1000 {
                thread::sleep(Duration::from_millis(5));
                pb.inc();
            }
        });
        t.spawn(|| {
            let pb = AvanceBar::new(800);
            pb.set_desc("task3");

            for _ in 0..800 {
                thread::sleep(Duration::from_millis(8));
                pb.inc();
            }
        });
    });
}
