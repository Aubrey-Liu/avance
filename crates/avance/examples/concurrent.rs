use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    let total = 1000;
    let mut v = vec![0; total];
    let pb = AvanceBar::new(total as u64);
    std::thread::scope(|t| {
        for chunk in v.chunks_mut(total / 4) {
            t.spawn(|| {
                for x in chunk {
                    *x = 1;

                    // Suppose we're doing some io tasks
                    thread::sleep(Duration::from_millis(2));

                    pb.inc();
                }
            });
        }
    });
    pb.close();

    let pb = AvanceBar::new(total as u64);
    std::thread::scope(|t| {
        t.spawn(|| {
            for x in &mut v {
                *x = 2;

                thread::sleep(Duration::from_millis(2));

                pb.inc();
            }
        });
    });
}
