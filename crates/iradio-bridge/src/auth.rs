use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::Engine;

/// Optional HTTP Basic Auth middleware.
/// Pass `enabled=false` to create a no-op passthrough layer.
pub async fn basic_auth_middleware(
    req: Request,
    next: Next,
    username: String,
    password: String,
    enabled: bool,
) -> Result<Response, StatusCode> {
    if !enabled {
        return Ok(next.run(req).await);
    }

    // Health endpoint is always public
    if req.uri().path() == "/api/v1/health" {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(auth) = auth_header {
        if let Some(encoded) = auth.strip_prefix("Basic ") {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(creds) = std::str::from_utf8(&decoded) {
                    if let Some((u, p)) = creds.split_once(':') {
                        if u == username && p == password {
                            return Ok(next.run(req).await);
                        }
                    }
                }
            }
        }
    }

    let mut response = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, r#"Basic realm="Inferno Radio""#)
        .body(axum::body::Body::from(
            r#"{"error":"authentication required"}"#,
        ))
        .unwrap();
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    Ok(response)
}
