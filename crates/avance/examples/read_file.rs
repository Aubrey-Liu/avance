use avance::AvanceBar;
use std::fs::File;
use std::io::{BufRead, BufReader, Result};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let f = File::open("/usr/share/dict/words").unwrap();
    let n_bytes = f.metadata().unwrap().len();
    let pb = AvanceBar::new(n_bytes);
    let reader = BufReader::new(f);
    for line in reader.lines() {
        let line = line?;
        pb.update(line.len() as u64 + 1); // need to count the '\n'
        thread::sleep(Duration::from_micros(10));
    }

    Ok(())
}
