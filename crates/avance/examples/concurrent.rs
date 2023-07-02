use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    let total = 1000;
    let mut v = vec![0; total];
    let pb1 = AvanceBar::new(total as u64)
        .with_style(Style::Balloon)
        .with_desc("x8");
    std::thread::scope(|t| {
        for chunk in v.chunks_mut(total / 8) {
            t.spawn(|| {
                for x in chunk {
                    *x = 1;

                    // Suppose we're doing some io tasks
                    thread::sleep(Duration::from_millis(3));

                    // You can use one progress bar fearlessly in multiple threads
                    pb1.inc();
                }
            });
        }
    });
    pb1.close();

    let pb2 = AvanceBar::with_config_of(&pb1).with_desc("x1");
    std::thread::scope(|t| {
        t.spawn(|| {
            for x in &mut v {
                *x = 2;

                thread::sleep(Duration::from_millis(3));

                pb2.inc();
            }
        });
    });
}
