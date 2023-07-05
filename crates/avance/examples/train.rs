use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    // Suppose we're training a model and here's 200 epoches
    let epoch = 200;
    let pb = AvanceBar::new(epoch).with_style(Style::Block);
    let mut accuracy = 30.0;

    for _ in 0..epoch {
        accuracy += 0.3;
        pb.set_postfix(format!("acc={:.2}", accuracy));
        pb.inc();

        thread::sleep(Duration::from_millis(20));
    }
}
