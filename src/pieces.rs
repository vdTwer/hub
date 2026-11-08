//! The registry of showcase pieces. Add a piece by appending to [`PIECES`]
//! and dropping its fragment shader into `static/shaders/`.

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

/// One showcase piece: a full-screen fragment shader with a short caption.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub slug: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// File name inside `static/shaders/`.
    pub shader: &'static str,
    /// Empty for a full-screen piece; otherwise one small canvas per card.
    pub cards: &'static [Card],
}

pub const PIECES: &[Piece] = &[
    Piece {
        slug: "fluid-grain",
        title: "Fluid, on film",
        blurb: "Domain-warped gradients that drift like ink in water, \
                developed onto a worn reel of 35mm stock.",
        shader: "fluid-grain.frag",
        cards: &[],
    },
    Piece {
        slug: "colorways",
        title: "Colorways",
        blurb: "The same fluid on film, poured six times, \
                each card around a different primary color.",
        shader: "fluid-grain.frag",
        cards: &COLORWAYS,
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
