//! Server-rendered showcase where sleek, modern visuals meet analog media.

pub mod calendar;
pub mod pieces;

use askama::Template;
use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use calendar::Week;
use pieces::{Card, Layout, PIECES, Piece};
use tower_http::services::ServeDir;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPage {
    pieces: &'static [Piece],
}

#[derive(Template)]
#[template(path = "piece.html")]
struct PiecePage {
    piece: &'static Piece,
}

#[derive(Template)]
#[template(path = "cards.html")]
struct CardsPage {
    piece: &'static Piece,
    cards: &'static [Card],
}

#[derive(Template)]
#[template(path = "calendar.html")]
struct CalendarPage {
    piece: &'static Piece,
    week: &'static Week,
}

fn render(template: &impl Template) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            eprintln!("template error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn index() -> Response {
    render(&IndexPage { pieces: PIECES })
}

async fn piece(Path(slug): Path<String>) -> Response {
    pieces::find(&slug).map_or_else(
        || (StatusCode::NOT_FOUND, "no such piece").into_response(),
        |piece| match piece.layout {
            Layout::Fullscreen => render(&PiecePage { piece }),
            Layout::Cards(cards) => render(&CardsPage { piece, cards }),
            Layout::Calendar(week) => render(&CalendarPage { piece, week }),
        },
    )
}

/// Builds the app, serving static assets from `static_dir`.
pub fn app(static_dir: &str) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/p/{slug}", get(piece))
        .nest_service("/static", ServeDir::new(static_dir))
}
