use anyhow::{anyhow, Error};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct AppError((Option<StatusCode>, Error));

impl AppError {
    pub fn new(err: Error) -> Self {
        Self((None, err))
    }

    pub fn status<E: Into<anyhow::Error>>(status: StatusCode, err: E) -> Self {
        Self((Some(status), err.into()))
    }

    pub fn not_found() -> Self {
        Self::status(StatusCode::NOT_FOUND, anyhow!("Not Found"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.0 .0.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            format!("{}", self.0 .1),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self((None, err.into()))
    }
}
