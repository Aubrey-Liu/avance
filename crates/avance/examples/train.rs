use std::thread;
use std::time::Duration;

use avance::*;

fn main() {
    // Suppose we're training a model and here's 200 epoches
    let epoch = 200;
    let mut accuracy = 30.0;

    (0..epoch)
        .avance()
        .with_style(Style::Block)
        .with_pb()
        .for_each(|(_, pb)| {
            thread::sleep(Duration::from_millis(20));

            accuracy += 0.34;

            // Display the accuracy through the postfix
            pb.set_postfix(format!("acc={:.2}", accuracy));
        });
}
