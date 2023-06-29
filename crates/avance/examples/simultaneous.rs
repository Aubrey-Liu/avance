use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    let pb = AvanceBar::new(1200);
    std::thread::scope(|t| {
        t.spawn(|| {
            for _ in 0..200 {
                thread::sleep(Duration::from_millis(3));
                pb.inc();
            }
        });
        t.spawn(|| {
            for _ in 0..400 {
                thread::sleep(Duration::from_millis(3));
                pb.inc();
            }
        });
        t.spawn(|| {
            for _ in 0..600 {
                thread::sleep(Duration::from_millis(3));
                pb.inc();
            }
        });
    });
}
