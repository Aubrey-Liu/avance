#![allow(unused)]
use clap::{Arg, Command};
use tqrs::tqrs;
use version::version;

fn main() {
    let matches = Command::new("tqrs")
        .version(version::version!())
        .arg(
            Arg::new("delim")
                .long("delim")
                .default_value("\n")
                .hide_default_value(true)
                .help(
                    "chr, optional 
Delimiting character [default: '\\n']. Use '\\0' for null.
N.B.: on Windows systems, Python converts '\\n' to '\\r\\n'.",
                ),
        )
        .arg(Arg::new("total").long("total").help(
            r#"int or float, optional
The number of expected iterations. If unspecified, tqrs will try
to 
len(iterable) is used if possible. If float("inf") or as a last
resort, only basic progress statistics are displayed
(no ETA, no progressbar)."#,
        ))
        .get_matches();
}
