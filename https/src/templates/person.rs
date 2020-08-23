use askama::Template;

#[derive(Template)]
#[template(path = "gmi_page.html")]
pub struct GmiPageTemplate<'a> {
  title: &'a str,
  html_block: &'a str,
}

impl<'a> GmiPageTemplate<'a> {
  pub fn new(title: &'a str, html_block: &'a str) -> Self {
    Self { title, html_block }
  }
}
