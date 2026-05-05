use axum::{
    body::Body,
    http::{Response, StatusCode, Uri, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

/// A handler that serves static assets from the embedded filesystem.
///
/// This handles paths like `/static/birthdays/list.js` by looking for
/// `birthdays/list.js` in the `static/` folder.
///
/// It also provides PWA support by falling back to `index.html` for
/// navigation requests that do not point to a specific file.
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Strip the "static/" prefix if it exists in the URL to find the file in the embed
    let asset_path = path.strip_prefix("static/").unwrap_or(path);

    // If the path is empty (the root), default to index.html
    let final_path = if asset_path.is_empty() {
        "index.html"
    } else {
        asset_path
    };

    match Assets::get(final_path) {
        Some(content) => {
            let mime = mime_guess::from_path(final_path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // PWA Fallback logic:
            // If the request doesn't contain a dot (likely a route like /birthdays/list),
            // serve index.html to let the frontend router handle it.
            if !final_path.contains('.') {
                if let Some(index) = Assets::get("index.html") {
                    return Response::builder()
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(index.data))
                        .unwrap();
                }
            }

            StatusCode::NOT_FOUND.into_response()
        }
    }
}
