use super::super::Config;
use askama::Template;

#[derive(Template)]
#[template(path = "sign/up.html")]
pub struct UpTemplate<'a> {
  config: &'a Config,
  errors: &'a [&'a str],
}

impl<'a> UpTemplate<'a> {
  pub fn new(config: &'a Config, errors: &'a [&'a str]) -> Self {
    Self { config, errors }
  }
}

#[derive(Template)]
#[template(path = "sign/in.html")]
pub struct InTemplate<'a> {
  config: &'a Config,
  error: Option<&'a str>,
}

impl<'a> InTemplate<'a> {
  pub fn new(config: &'a Config, error: Option<&'a str>) -> Self {
    Self { config, error }
  }
}

#[derive(Template)]
#[template(path = "sign/out.html")]
pub struct OutTemplate {}
impl OutTemplate {
  pub fn new() -> Self {
    Self {}
  }
}
