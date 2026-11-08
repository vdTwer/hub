//! The registry of showcase pieces. Add a piece by appending to [`PIECES`]
//! and dropping its fragment shader into `static/shaders/`.

use crate::calendar::{WEEK, Week};

/// One colorway for a card piece. The shader reads the colors in order as
/// base, soft, primary, accent and secondary.
#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub name: &'static str,
    pub palette: [&'static str; 5],
}

impl Card {
    /// The color the card is named after.
    #[must_use]
    pub const fn primary(&self) -> &'static str {
        self.palette[2]
    }
}

/// How a piece is laid out on its page.
#[derive(Debug, Clone, Copy)]
pub enum Layout {
    /// One canvas filling the window.
    Fullscreen,
    /// One small canvas per card.
    Cards(&'static [Card]),
    /// An app mockup: a calendar week, with a still print of the shader
    /// behind days that differ.
    Calendar(&'static Week),
}

/// One showcase piece: a layout, a short caption and usually a shader.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub slug: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// File name inside `static/shaders/`, if the piece runs one.
    pub shader: Option<&'static str>,
    pub layout: Layout,
}

impl Piece {
    /// The URL of the piece's shader, or an empty string without one.
    #[must_use]
    pub fn shader_url(&self) -> String {
        self.shader
            .map(|s| format!("/static/shaders/{s}"))
            .unwrap_or_default()
    }

    /// The piece's cards, empty unless it is a card piece.
    #[must_use]
    pub const fn cards(&self) -> &'static [Card] {
        match self.layout {
            Layout::Cards(cards) => cards,
            _ => &[],
        }
    }
}

pub const PIECES: &[Piece] = &[
    Piece {
        slug: "fluid-grain",
        title: "Fluid, on film",
        blurb: "Domain-warped gradients that drift like ink in water, \
                developed onto a worn reel of 35mm stock.",
        shader: Some("fluid-grain.frag"),
        layout: Layout::Fullscreen,
    },
    Piece {
        slug: "colorways",
        title: "Colorways",
        blurb: "The same fluid on film, poured six times, \
                each card around a different primary color.",
        shader: Some("fluid-grain.frag"),
        layout: Layout::Cards(&COLORWAYS),
    },
    Piece {
        slug: "calendar",
        title: "A week, on paper",
        blurb: "A calendar mockup: crisp events and todos on a paper week, \
                with unusual days washed in grainy fluid color.",
        shader: Some("fluid-grain.frag"),
        layout: Layout::Calendar(&WEEK),
    },
];

const COLORWAYS: [Card; 6] = [
    Card {
        name: "Cobalt",
        palette: ["#d9e0ff", "#7f9bff", "#1f3be0", "#63c7ff", "#5a3fe8"],
    },
    Card {
        name: "Vermilion",
        palette: ["#ffe1d6", "#ff8a6b", "#e3271f", "#ffb238", "#ff4f7b"],
    },
    Card {
        name: "Emerald",
        palette: ["#dff3e4", "#6fd39a", "#08905a", "#c9ec5a", "#1aa9a1"],
    },
    Card {
        name: "Saffron",
        palette: ["#fff0c9", "#ffd257", "#f2a900", "#ff6a2b", "#e07b12"],
    },
    Card {
        name: "Magenta",
        palette: ["#fbe0f0", "#f58ccc", "#cf1f7a", "#8a4dff", "#ff6f9e"],
    },
    Card {
        name: "Lagoon",
        palette: ["#d8f2f1", "#6fd6d0", "#078a99", "#4e6fff", "#2fc7a4"],
    },
];

#[must_use]
pub fn find(slug: &str) -> Option<&'static Piece> {
    PIECES.iter().find(|p| p.slug == slug)
}
