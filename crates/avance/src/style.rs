//! Styles of a progress bar

use strum::AsRefStr;

#[derive(Debug, Clone, Copy, Default, AsRefStr)]
pub enum Style {
    #[default]
    #[strum(serialize = "0123456789#")]
    ASCII,
    #[strum(serialize = " ▏▎▍▌▋▊▉█")]
    Block,
    #[strum(serialize = ".oO@*")]
    Balloon,
}
