use ntex::web;
use thiserror::Error;
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