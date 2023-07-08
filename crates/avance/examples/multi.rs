use std::thread;
use std::time::Duration;

use avance::{AvanceBar, Style};

fn main() {
    std::thread::scope(|t| {
        t.spawn(|| {
            AvanceBar::new(1200)
                .with_desc("default")
                .with_iter(0..1200)
                .for_each(|_| thread::sleep(Duration::from_millis(3)));
        });
        t.spawn(|| {
            AvanceBar::new(1000)
                .with_style(Style::Balloon)
                .with_desc("balloon")
                .with_iter(0..1000)
                .for_each(|_| thread::sleep(Duration::from_millis(5)));
        });
        t.spawn(|| {
            AvanceBar::new(800)
                .with_style_str("=>-") // user-defined style
                .with_desc("custom")
                .with_iter(0..800)
                .for_each(|_| thread::sleep(Duration::from_millis(8)));
        });
    });
}
