use super::State;
use crate::records::{File, Person};
use crate::templates::edit::*;
use crate::utils::{deserialize_body, GetPerson};
use serde::Deserialize;
use sqlx::prelude::SqliteQueryAs;
use std::path::Path;
use tide::{Error, StatusCode};

#[derive(Deserialize)]
struct EditFileForm {
  file_text: String,
}

pub async fn get_edit(request: crate::Request) -> tide::Result {
  let State { db: _, config } = request.state();
  let file_name = request
    .param::<String>("file_name")
    .map_err(|_| Error::from_str(StatusCode::BadRequest, "you can't edit nothing homie"))?;
  let person: Person = request.get_person()?;
  let full_path = File::get_full_path(&config.file_directory, &person.username, &file_name);
  let file_text = std::fs::read_to_string(full_path).unwrap_or_else(|_| "".into());
  Ok(IndexTemplate::new(&file_name, &file_text, &person, config).into())
}

pub async fn post_edit(mut request: crate::Request) -> tide::Result {
  let State { config, db } = request.state().clone();
  let person: Person = request.get_person()?;
  let file_form = deserialize_body::<EditFileForm>(&mut request).await?;
  let file_text = file_form.file_text.as_bytes();
  let file_name = request
    .param::<String>("file_name")
    .map_err(|_| Error::from_str(StatusCode::BadRequest, "you can't edit nothing homie"))?;
  let sanitized_file = &sanitize_filename::sanitize(&file_name);
  let full_path = File::get_full_path(&config.file_directory, &person.username, &file_name);
  let path_str = full_path.to_str().ok_or_else(|| {
    Error::from_str(
      StatusCode::InternalServerError,
      "couldn't convert path to string",
    )
  })?;
  File::upsert(sanitized_file, person.id, &path_str)
    .execute(&db)
    .await?;
  File::write_all(file_text, full_path)?;
  Ok(tide::Redirect::new("/my_site").into())
}

pub async fn delete(request: crate::Request) -> tide::Result {
  let State { db, config } = request.state();

  let person = request.get_person()?;

  let req_path = request.param::<String>("file_name")?;
  let filename = sanitize_filename::sanitize(req_path);
  let full_path = Path::new(&config.file_directory)
    .join(&person.username)
    .join(filename);
  if !full_path.exists() {
    return Err(Error::from_str(
      StatusCode::NotFound,
      "that file doesn't exist",
    ));
  }
  File::delete_from_fs(&full_path);
  File::delete_by_path(full_path.to_str().ok_or_else(|| {
    Error::from_str(
      StatusCode::InternalServerError,
      "couldn't get path as string??",
    )
  })?)
  .execute(db)
  .await?;
  Ok(tide::Redirect::new("/my_site").into())
}

pub async fn post_upload(mut request: crate::Request) -> tide::Result {
  let State { config, db } = request.state().clone();
  let person: Person = request
    .session()
    .get("person")
    .ok_or_else(|| Error::from_str(StatusCode::Unauthorized, "yo you're not logged in"))?;

  let (count,) = File::count_for_person(person.id).fetch_one(&db).await?;
  if count >= 128 {
    return Err(Error::from_str(
      StatusCode::InsufficientStorage,
      "you got too many files bud— try deleting some?",
    ));
  }

  let file_directory: String = config.file_directory.clone();
  let stream = request.take_body();
  let content_type = request
    .content_type()
    .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "no content type"))?;
  let boundary = content_type
    .param("boundary")
    .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "no boundary param on content type"))?
    .as_str();

  File::stream_in_multipart(stream, boundary, &file_directory, &person, &db).await
}
