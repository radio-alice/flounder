use actix_web::{error::ErrorInternalServerError, Error, HttpResponse};
use askama::*;
use bytes::BytesMut;

pub trait TemplateIntoResponse {
    fn into_response(&self) -> ::std::result::Result<HttpResponse, Error>;
}

impl<T: askama::Template> TemplateIntoResponse for T {
    fn into_response(&self) -> ::std::result::Result<HttpResponse, Error> {
        let mut buffer = BytesMut::with_capacity(self.size_hint());
        self.render_into(&mut buffer)
            .map_err(|_| ErrorInternalServerError("Template parsing error"))?;

        let ctype = "text/html";
        Ok(HttpResponse::Ok().content_type(ctype).body(buffer.freeze()))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub logged_in: bool,
    pub files: Vec<RenderedFile>, // arr?
}

pub struct RenderedFile {
    pub username: String,
    pub user_path: String,
    pub updated_at: u32,
}

#[derive(Template)]
#[template(path = "my_site.html")]
pub struct MySiteTemplate {
    pub logged_in: bool,
    pub files: Vec<RenderedFile>, // arr?
}
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    // errors?
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {}

#[derive(Template)]
#[template(path = "edit_file.html")]
pub struct EditFileTemplate {
    pub filename: String,
    pub file_text: String,
}
