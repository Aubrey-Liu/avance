//! Styles of a progress bar

use std::borrow::Cow;

/// Styles of a progress bar
#[derive(Debug, Clone, Default)]
pub enum Style {
    /// Presentation: `|######7             |`
    #[default]
    ASCII,

    /// Presentation: `|███████             |`
    Block,

    /// Presentation: `|******@             |`
    Balloon,

    /// User custom style
    Custom(Cow<'static, str>),
}

impl AsRef<str> for Style {
    fn as_ref(&self) -> &str {
        match self {
            Self::ASCII => "#0123456789 ",
            Self::Block => "█ ▏▎▍▌▋▊▉ ",
            Self::Balloon => "*.oO@ ",
            Self::Custom(s) => s,
        }
    }
}
