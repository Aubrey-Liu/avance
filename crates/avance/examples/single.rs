use std::thread;
use std::time::Duration;

use avance::AvanceIterator;

fn main() {
    for _ in (0..1000).fuse().avance() {
        thread::sleep(Duration::from_millis(5));
    }
}
