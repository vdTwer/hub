//! The registry of showcase pieces. Add a piece by appending to [`PIECES`]
//! and dropping its fragment shader into `static/shaders/`.

/// One showcase piece: a full-screen fragment shader with a short caption.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub slug: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// File name inside `static/shaders/`.
    pub shader: &'static str,
}

pub const PIECES: &[Piece] = &[Piece {
    slug: "fluid-grain",
    title: "Fluid, on film",
    blurb: "Domain-warped gradients that drift like ink in water, \
            developed onto a worn reel of 35mm stock.",
    shader: "fluid-grain.frag",
}];

#[must_use]
pub fn find(slug: &str) -> Option<&'static Piece> {
    PIECES.iter().find(|p| p.slug == slug)
}
