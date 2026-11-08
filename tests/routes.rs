use axum::{body::Body, http::Request, http::StatusCode};
use design_showcase::{
    app,
    pieces::{PIECES, Piece},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

const STATIC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

async fn get(uri: &str) -> (StatusCode, String) {
    let request = Request::get(uri).body(Body::empty());
    let Ok(request) = request else {
        return (StatusCode::BAD_REQUEST, String::new());
    };
    let Ok(response) = app(STATIC).oneshot(request).await;
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .map(http_body_util::Collected::to_bytes)
        .unwrap_or_default();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn index_links_every_piece() {
    let (status, body) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    for piece in PIECES {
        assert!(
            body.contains(&format!("/p/{}", piece.slug)),
            "{}",
            piece.slug
        );
    }
}

#[tokio::test]
async fn every_piece_page_and_shader_is_served() {
    for piece in PIECES {
        let (status, body) = get(&format!("/p/{}", piece.slug)).await;
        assert_eq!(status, StatusCode::OK);
        let Some(file) = piece.shader else {
            assert!(!body.contains("data-shader"), "{}", piece.slug);
            continue;
        };
        assert!(body.contains(file));

        let (status, shader) = get(&format!("/static/shaders/{file}")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(shader.starts_with("#version 300 es"));
    }
}

#[tokio::test]
async fn unknown_piece_is_404() {
    let (status, _) = get("/p/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stage_script_is_served() {
    let (status, _) = get("/static/js/stage.js").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn every_app_offers_the_darkroom_toggle() {
    let (status, _) = get("/static/js/theme.js").await;
    assert_eq!(status, StatusCode::OK);
    for slug in ["calendar", "mail", "contacts"] {
        let (_, body) = get(&format!("/p/{slug}")).await;
        assert!(
            body.contains("/static/js/theme.js"),
            "{slug} lacks theme.js"
        );
        assert!(body.contains("theme-toggle"), "{slug} lacks the toggle");
    }
}

#[tokio::test]
async fn card_pieces_render_one_canvas_per_card() {
    for piece in PIECES.iter().filter(|p| !p.cards().is_empty()) {
        let (status, body) = get(&format!("/p/{}", piece.slug)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.matches("data-palette=").count(), piece.cards().len());
        for card in piece.cards() {
            assert!(body.contains(card.name), "{}", card.name);
            assert!(body.contains(&card.palette.join(",")), "{}", card.name);
        }
    }
}

#[test]
fn palettes_are_five_hex_colors() {
    for card in PIECES.iter().flat_map(Piece::cards) {
        for color in card.palette {
            let hex = color.strip_prefix('#').unwrap_or_default();
            assert!(
                hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()),
                "{}: {color}",
                card.name
            );
        }
    }
}

#[tokio::test]
async fn calendar_shows_every_feature() {
    let (status, body) = get("/p/calendar").await;
    assert_eq!(status, StatusCode::OK);
    for needle in [
        "class=\"allday",
        "multi-day",
        "class=\"event",
        "class=\"todo",
        "Sometime this week",
        "data-palette=",
        "data-still",
        "now-line",
        "is-now",
    ] {
        assert!(body.contains(needle), "missing {needle}");
    }
}

#[tokio::test]
async fn mail_shows_every_feature() {
    let (status, body) = get("/p/mail").await;
    assert_eq!(status, StatusCode::OK);
    for needle in [
        "Pinned",
        "row unread",
        "row open",
        "class=\"owe",
        "class=\"clip",
        "class=\"folded",
        "class=\"stack",
        "class=\"new-line",
        "<blockquote",
        "meta-btn",
        "On your calendar",
        "tag plain",
        "class=\"files",
        "class=\"composer",
        "class=\"glass toolbar-group",
        "data-still",
        "data-palette=",
    ] {
        assert!(body.contains(needle), "missing {needle}");
    }
}

#[tokio::test]
async fn contacts_shows_every_feature() {
    let (status, body) = get("/p/contacts").await;
    assert_eq!(status, StatusCode::OK);
    for needle in [
        "class=\"group letter\">A<",
        "person work open",
        "<b>Lindqvist</b>",
        "Favorites",
        "class=\"details",
        "class=\"threads",
        "Off-site in Berlin",
        "Quick sync with Ava",
        "tag plain",
        "data-still",
        "data-palette=",
    ] {
        assert!(body.contains(needle), "missing {needle}");
    }
}
