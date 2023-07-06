use std::cmp::min;
use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    let n_bytes = 1024 * 1024;
    let mut bytes_read = 0;

    let pb = AvanceBar::new(n_bytes)
        .with_style(Style::Block)
        .with_desc("reading");

    while bytes_read < n_bytes {
        bytes_read = min(bytes_read + 1378, n_bytes);
        pb.update(1378);
        thread::sleep(Duration::from_millis(5));
    }

    pb.set_desc("done");
}
