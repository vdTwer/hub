use axum::{body::Body, http::Request, http::StatusCode};
use design_showcase::{app, pieces::PIECES};
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
        assert!(body.contains(piece.shader));

        let (status, shader) = get(&format!("/static/shaders/{}", piece.shader)).await;
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
