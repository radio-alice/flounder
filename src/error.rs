use actix_web::{web, App, Error, HttpResponse, HttpServer, ResponseError};
use derive_more::Display; // naming it clearly for illustration purposes

#[derive(Debug, Display)]
pub enum FlounderError {
    MiscError, // should not occur.
    UnauthorizedError,
}

/// Actix web uses `ResponseError` for conversion of errors to a response
impl ResponseError for FlounderError {
    fn error_response(&self) -> HttpResponse {
        match self {
            UnauthorizedError => HttpResponse::Forbidden().finish(),
            _ => HttpResponse::InternalServerError().finish()
            }
        }
    }


impl From<rusqlite::Error> for FlounderError {
    fn from(err: rusqlite::Error) -> FlounderError {
        FlounderError::MiscError
    }
}

impl From<Error> for FlounderError { 
    fn from(err: Error) -> FlounderError {
        FlounderError::MiscError
    }
}

impl From<std::io::Error> for FlounderError { 
    fn from(err: std::io::Error) -> FlounderError {
        FlounderError::MiscError
    }
}

impl From<actix_multipart::MultipartError> for FlounderError { 
    fn from(err: actix_multipart::MultipartError) -> FlounderError {
        FlounderError::MiscError
    }
}
