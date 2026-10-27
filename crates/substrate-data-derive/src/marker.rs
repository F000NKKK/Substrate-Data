//! [`Marker`] and [`Fallback`]: the `#[data(...)]` options that aren't text.

/// An option that takes no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Marker {
    /// `skip`: never read or written (`Default`).
    Skip,
    /// `other`: a tagged enum's variant for every tag the others don't take.
    Other,
    /// `view`: a struct gets a view of frames (`substrate_data::frame::View`).
    View,
}

/// What an optional field is when it isn't there (`default`).
pub(crate) enum Fallback {
    /// `Default::default()`.
    Default,
    /// `default = "path"`: what `path()` makes.
    Call(syn::Path),
}
