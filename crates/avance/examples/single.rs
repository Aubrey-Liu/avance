use std::thread;
use std::time::Duration;

use avance::AvanceBar;

fn main() {
    let bar = AvanceBar::new(1000);
    for _ in 0..1000 {
        bar.update(1);
        thread::sleep(Duration::from_millis(5));
    }
}
