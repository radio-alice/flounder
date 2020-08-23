use super::{Query, QueryAs};
use crate::records::Person;
use async_std::stream::Stream;
use async_std::task::{Context, Poll};
use futures::io::AsyncRead;
use multer::Multipart;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tide::{Error, StatusCode};

#[derive(sqlx::FromRow, Deserialize, Serialize)]
pub struct File {
  pub id: i64,
  pub user_path: String,
  pub full_path: String,
  pub user_id: i64,
  created_at: i32,
  updated_at: i32,
}

#[derive(sqlx::FromRow)]
pub struct PartialFile {
  pub username: String,
  pub user_path: String,
  pub updated_at: i32,
}

// this looks weird but it's the only way the borrow checker would have it
pub struct RenderedFile<'a> {
  pub username: &'a str,
  pub user_path: &'a str,
  pub time_ago: String,
}

impl crate::utils::AsRoute for File {
  fn as_route(&self) -> std::borrow::Cow<str> {
    format!("/{}", self.user_path).into()
  }
}

static ALLOWED_EXTENSIONS: &[&str] = &[
  "gmi", "txt", "jpg", "jpeg", "gif", "png", "svg", "webp", "midi", "json", "csv", "gemini", "mp3",
];

static TEXT_EXTENSIONS: &[&str] = &["gmi", "txt", "json", "csv", "gemini"];

impl File {
  pub fn ok_extension(filename: &str) -> bool {
    let tmp = filename.to_lowercase();
    let lower_extension: Option<&str> = Path::new(&tmp).extension().and_then(|s| s.to_str());
    ALLOWED_EXTENSIONS
      .iter()
      .any(|s| Some(*s) == lower_extension)
  }

  pub fn text_extension(&self) -> bool {
    let tmp = &self.user_path.to_lowercase();
    let lower_extension: Option<&str> = Path::new(&tmp).extension().and_then(|s| s.to_str());
    TEXT_EXTENSIONS.iter().any(|s| Some(*s) == lower_extension)
  }

  pub fn get_full_path(file_dir: &str, username: &str, user_path: &str) -> PathBuf {
    let sanitized_file_name = &sanitize_filename::sanitize(user_path);
    Path::new(file_dir).join(username).join(sanitized_file_name)
  }

  pub fn get_recent() -> QueryAs<PartialFile> {
    sqlx::query_as(
      "SELECT user.username, file.user_path, file.updated_at
        FROM file
        JOIN user
        ON file.user_id = user.id
        ORDER BY file.updated_at DESC
        LIMIT 32",
    )
  }

  pub fn count_for_person(user_id: i64) -> QueryAs<(i32,)> {
    sqlx::query_as("SELECT COUNT(*) FROM file where user_id = ?").bind(user_id)
  }

  pub fn all_for_person(user_id: i64) -> QueryAs<Self> {
    sqlx::query_as("SELECT * FROM file where user_id = ?").bind(user_id)
  }

  pub fn delete_from_fs(full_path: &Path) {
    std::fs::remove_file(full_path).ok();
  }

  pub fn delete_by_path(full_path: &str) -> Query {
    sqlx::query("DELETE FROM file where full_path = ?").bind(full_path)
  }

  pub fn upsert(local_path: &str, user_id: i64, full_path: &str) -> Query {
    sqlx::query(
      "INSERT INTO file (user_path, user_id, full_path)
      VALUES ($1, $2, $3)
      ON CONFLICT(full_path) DO UPDATE SET
      updated_at=strftime('%s', 'now')",
    )
    .bind(local_path)
    .bind(user_id)
    .bind(full_path)
  }

  pub fn write_all(data: &[u8], full_path: PathBuf) -> tide::Result {
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(true)
      .open(&full_path)?;
    file.write_all(data)?;
    // just to satisfy tide::Result
    Ok(StatusCode::Ok.into())
  }

  // TODO - remove all this in favor of tide-native
  // multipart handling once it's released
  pub async fn stream_in_multipart(
    stream: tide::Body,
    boundary: &str,
    file_dir: &str,
    person: &Person,
    db: &SqlitePool,
  ) -> tide::Result {
    let mut multipart = Multipart::new(ByteStream(stream), boundary);

    while let Some(mut field) = multipart.next_field().await? {
      let file_name = field
        .file_name()
        .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "no file name"))?;
      if !File::ok_extension(file_name) {
        return Err(Error::from_str(
          StatusCode::BadRequest,
          "We don't take that kind of file here",
        ));
      }
      let sanitized_file_name = &sanitize_filename::sanitize(file_name);
      let full_path = File::get_full_path(file_dir, &person.username, file_name);

      // create user dir if it doesn't exist
      if !full_path.exists() {
        person
          .create_dir(file_dir)
          .map_err(|e| Error::from_str(StatusCode::InternalServerError, e))?;
      }

      let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&full_path)?;

      while let Some(chunk) = field.chunk().await? {
        file.write_all(&chunk)?;
      }

      let path_str = full_path.to_str().ok_or_else(|| {
        Error::from_str(
          StatusCode::InternalServerError,
          "couldn't convert path to string",
        )
      })?;
      Self::upsert(sanitized_file_name, person.id, &path_str)
        .execute(db)
        .await?;
    }
    Ok(tide::Redirect::new("/my_site").into())
  }
}

pub struct ByteStream<R>(R);

impl<R: Unpin + AsyncRead> Stream for ByteStream<R> {
  type Item = Result<Vec<u8>, futures::io::Error>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut buf = [0u8; 1024];
    match Pin::new(&mut self.0).poll_read(cx, &mut buf[..]) {
      Poll::Pending => Poll::Pending,
      Poll::Ready(Ok(0)) => Poll::Ready(None),
      Poll::Ready(Ok(n)) => Poll::Ready(Some(Ok(buf[..n].to_vec()))),
      Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
    }
  }
}
