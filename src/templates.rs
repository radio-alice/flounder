use actix_web::{error::ErrorInternalServerError, HttpResponse};
use askama::*;
use bytes::BytesMut;

use crate::error::FlounderError;
use crate::twtxt::TwtxtStatus;

pub trait TemplateIntoResponse {
    fn into_response(&self) -> ::std::result::Result<HttpResponse, FlounderError>;
}

impl<T: askama::Template> TemplateIntoResponse for T {
    fn into_response(&self) -> std::result::Result<HttpResponse, FlounderError> {
        let mut buffer = BytesMut::with_capacity(self.size_hint());
        self.render_into(&mut buffer)
            .map_err(|_| ErrorInternalServerError("Template parsing error"))?;

        let ctype = "text/html";
        Ok(HttpResponse::Ok().content_type(ctype).body(buffer.freeze()))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub logged_in: bool,
    pub server_name: &'a str,
    pub files: Vec<RenderedFile>, // arr?
    pub users: Vec<String>,
}

pub struct RenderedFile {
    pub username: String,
    pub user_path: String,
    pub time_ago: String,
}

#[derive(Template)]
#[template(path = "my_site.html")]
pub struct MySiteTemplate<'a> {
    pub logged_in: bool,
    pub server_name: &'a str,
    pub username: &'a str,
    pub files: Vec<RenderedFile>, // arr?
    pub errors: Vec<String>,
}
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    pub errors: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate<'a> {
    pub server_name: &'a str,
    pub errors: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "edit_file.html")]
pub struct EditFileTemplate {
    pub filename: String,
    pub file_text: String,
}

#[derive(Template)]
#[template(path = "gmi_page.html")]
pub struct GmiPageTemplate<'a> {
    pub title: &'a str,
    pub html_block: &'a str,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub error: String,
}

#[derive(Template)]
#[template(path = "statuses.html")]
pub struct StatusesTemplate<'a> {
    pub logged_in: bool,
    pub statuses: Vec<TwtxtStatus>,
    pub server_name: &'a str,
}
