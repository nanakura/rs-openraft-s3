use thiserror::Error;
use ntex::web;
#[derive(Error, Debug)]
pub enum AppError {
    #[error("error: `{0}`")]
    Anyhow(#[from] anyhow::Error),
    #[error("not found")]
    NotFound,
    #[error("bad request")]
    BadRequest,
}

impl web::error::WebResponseError for AppError {}