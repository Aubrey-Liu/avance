use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    std::thread::scope(|t| {
        t.spawn(|| {
            let bar = AvanceBar::new(1200);
            bar.set_description("task1");

            for _ in 0..1200 {
                thread::sleep(Duration::from_millis(3));
                bar.update(1)
            }
        });
        t.spawn(|| {
            let bar = AvanceBar::new(1000);
            bar.set_description("task2");

            for _ in 0..1000 {
                thread::sleep(Duration::from_millis(5));
                bar.update(1)
            }
        });
        t.spawn(|| {
            let bar = AvanceBar::new(800);
            bar.set_description("task3");

            for _ in 0..800 {
                thread::sleep(Duration::from_millis(8));
                bar.update(1)
            }
        });
    });
}
