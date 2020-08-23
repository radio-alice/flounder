use super::super::Config;
use crate::records::Person;
use askama::Template;

#[derive(Template)]
#[template(path = "edit_file.html")]
pub struct IndexTemplate<'a> {
  filename: &'a str,
  file_text: &'a str,
  person: Option<&'a Person>,
  config: &'a Config,
}

impl<'a> IndexTemplate<'a> {
  pub fn new(
    filename: &'a str,
    file_text: &'a str,
    person: &'a Person,
    config: &'a Config,
  ) -> Self {
    Self {
      filename,
      file_text,
      person: Some(person),
      config,
    }
  }
}
