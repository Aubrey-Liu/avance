//! Styles of a progress bar

use strum::AsRefStr;

/// Styles of a progress bar
///
/// An abstract pattern is like "{Filling} {In-progress} {Background}" (left to right):
/// - Background (the first character)
/// - Filling (the last character)
/// - In-progress Unit, which is the rightmost unit of the filled part (Use other characters in order)
///
/// Take " 0123456789#" as an example, the background is the blank,
/// the filling is '#', and "0123456789" is used for the in-progress unit.
#[derive(Debug, Clone, Copy, Default, AsRefStr)]
pub enum Style {
    /// Presentation: `|######7             |`
    #[default]
    #[strum(serialize = " 0123456789#")]
    ASCII,

    /// Presentation: `|███████             |`
    #[strum(serialize = "  ▏▎▍▌▋▊▉█")]
    Block,

    /// Presentation: `|******@             |`
    #[strum(serialize = " .oO@*")]
    Balloon,
}
