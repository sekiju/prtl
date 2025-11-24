use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum ApiError {
    InvalidPath,
    InvalidUrl(String),
    NoParserAvailable,
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::InvalidPath => (StatusCode::BAD_REQUEST, "Invalid path".to_string()),
            ApiError::InvalidUrl(msg) => (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", msg)),
            ApiError::NoParserAvailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "No proxy available for this domain".to_string(),
            ),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {}", msg)),
        };

        (status, message).into_response()
    }
}
