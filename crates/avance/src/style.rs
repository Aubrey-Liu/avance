//! Progress bar style enumeration
//!
//! - `ASCII`: Pure ASCII bar with `"0123456789#"`
//! - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`
//! - `Balloon`: Simulate balloon explosion with `".oO@*"`.
//!

#[derive(Default)]
pub enum Style {
    #[default]
    ASCII,
    Block,
    Balloon,
}

impl ToString for Style {
    fn to_string(&self) -> String {
        String::from(match self {
            Style::ASCII => "0123456789#",
            Style::Block => " ▏▎▍▌▋▊▉█",
            Style::Balloon => ".oO@*",
        })
    }
}
