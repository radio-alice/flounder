use super::super::State;
use crate::records::File;
use crate::templates::person::*;
use crate::utils::gemtext2html::*;

use serde::Deserialize;
use std::ffi::OsStr;
use tide::{Error, StatusCode};

#[derive(Deserialize)]
struct RawParam {
  raw: u8,
}

pub async fn get(request: crate::Request) -> tide::Result {
  let State { config, db: _ } = request.state();
  let username = request.param::<String>("person")?;
  let file_name = request
    .param::<String>("file_name")
    .unwrap_or_else(|_| "index.gmi".into());

  let full_path = File::get_full_path(&config.file_directory, &username, &file_name);

  if full_path.extension() == Some(OsStr::new("gmi"))
    || full_path.extension() == Some(OsStr::new("gemini"))
  {
    let gmi_file = std::fs::read_to_string(full_path)
      .map_err(|_| Error::from_str(StatusCode::NotFound, "not here buddy"))?;
    if let Ok(RawParam { raw: 1 }) = request.query::<RawParam>() {
      return Ok(tide::Response::from(gmi_file));
    }

    let html_block = gemtext_to_html(&gmi_file);
    Ok(GmiPageTemplate::new(&file_name, &html_block).into())
  } else if full_path.exists() {
    Ok(tide::Body::from_file(full_path).await?.into())
  } else {
    Ok(Error::from_str(StatusCode::NotFound, "not here bucko").into())
  }
}
