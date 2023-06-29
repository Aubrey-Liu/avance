use avance::AvanceBar;
use std::cmp::min;
use std::thread;
use std::time::Duration;

fn main() {
    let n_bytes = 1024 * 1024;
    let mut bytes_read = 0;

    let pb = AvanceBar::new(n_bytes);
    pb.set_style(avance::Style::Block);
    pb.set_description("reading");

    while bytes_read < n_bytes {
        bytes_read = min(bytes_read + 1378, n_bytes);
        pb.update(1378);
        thread::sleep(Duration::from_millis(5));
    }

    pb.set_description("done");
}
