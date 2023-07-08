use std::thread;
use std::time::Duration;

use avance::AvanceIterator;

fn main() {
    (0..1000)
        .avance()
        .for_each(|_| thread::sleep(Duration::from_millis(5)));
}
