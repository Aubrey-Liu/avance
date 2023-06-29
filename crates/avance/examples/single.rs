use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    let pb = AvanceBar::new(1000);
    for _ in 0..1000 {
        pb.inc();
        thread::sleep(Duration::from_millis(5));
    }
}
