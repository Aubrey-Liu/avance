use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    // Suppose we're training a model and here's 200 epoches
    let epoch = 200;
    let mut accuracy = 30.0;

    for (_, pb) in (0..epoch).avance().with_style(Style::Block).with_pb() {
        accuracy += 0.3;
        pb.set_postfix(format!("acc={:.2}", accuracy));

        thread::sleep(Duration::from_millis(20));
    }
}
