//! Styles of a progress bar

use strum::AsRefStr;

/// Styles of a progress bar
#[derive(Debug, Clone, Copy, Default, AsRefStr)]
pub enum Style {
    // Pattern Format: "{background}{filling characters}"
    #[default]
    #[strum(serialize = " 0123456789#")]
    ASCII,
    #[strum(serialize = "  ▏▎▍▌▋▊▉█")]
    Block,
    #[strum(serialize = " .oO@*")]
    Balloon,
}
