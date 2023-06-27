#![allow(unused)]
use avance::avance;
use clap::{Arg, Command};
use version::version;

fn main() {
    let matches = Command::new("avc")
        .version(version::version!())
        .arg(
            Arg::new("delim")
                .long("delim")
                .default_value("\n")
                .hide_default_value(true)
                .help(
                    "chr, optional
Delimiting character [default: '\\n'].
                ",
                ),
        )
        .arg(Arg::new("total").long("total").help(
            "int, optional
The number of expected iterations.
If unspecified, only basic progress bar are displayed",
        ))
        .get_matches();
}
