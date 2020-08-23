use super::QueryAs;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct NewPerson {
  username: String,
  email: String,
  password: String,
  password2: String,
}

impl NewPerson {
  pub fn validate(&self) -> Vec<&str> {
    let mut errors = vec![];
    if self.username.len() > 32
      || self.username == ""
      || &self.username.to_lowercase() == "www"
      || &self.username.to_lowercase() == "proxy"
    {
      errors.push("Reserved name");
    }
    if !self
      .username
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
      errors.push("Username must only contain a-z, 0-9 or '-'");
    }
    if !self.email.contains('@') {
      // world's dumbest email verification (we dont really use email)
      errors.push("");
    }
    if self.password != self.password2 {
      errors.push("Passwords do not match");
    }
    if self.password.len() < 6 {
      errors.push("OK can we maybe use a longer password");
    }
    errors
  }

  pub fn create(&self) -> QueryAs<(i64,)> {
    let password_hash = bcrypt::hash(&self.password, bcrypt::DEFAULT_COST).unwrap();
    sqlx::query_as(
      "INSERT INTO user (username, email, password_hash, created_at) VALUES (
      $1, $2, $3, strftime('%s', 'now'));
      SELECT last_insert_rowid() as id;",
    )
    .bind(&self.username)
    .bind(&self.email)
    .bind(password_hash)
  }
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct Person {
  pub id: i64,
  pub username: String,
  pub password_hash: String,
  created_at: i32,
}

impl crate::utils::AsRoute for Person {
  fn as_route(&self) -> std::borrow::Cow<str> {
    format!("/person/{}", self.username).into()
  }
}

impl Person {
  pub fn all_names() -> QueryAs<(String,)> {
    sqlx::query_as("SELECT username FROM user")
  }

  pub fn find_by_username(username: &str) -> QueryAs<Self> {
    sqlx::query_as("SELECT * FROM user WHERE username = ?").bind(username)
  }

  pub fn find_by_id(id: i64) -> QueryAs<Self> {
    sqlx::query_as("SELECT * FROM user WHERE id = ?").bind(id)
  }

  pub fn create_dir(&self, file_dir: &str) -> Result<()> {
    let user_dir = Path::join(Path::new(&file_dir), &self.username);
    if !user_dir.exists() {
      std::fs::create_dir_all(user_dir).map_err(|_| {
        anyhow!("user directory doesn't exist and we don't have the permissions to create it!")
      })?;
    }
    Ok(())
  }
}
