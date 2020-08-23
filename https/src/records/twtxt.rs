use super::QueryAs;
/// see https://twtxt.readthedocs.io/en/latest/user/twtxtfile.html
use crate::utils::rendered_time_ago;
use chrono::{DateTime, NaiveDateTime};

pub struct TwtxtStatus {
  pub date: NaiveDateTime,
  pub time_ago: String,
  pub username: String,
  pub text: String, // TODO figure out str
}

impl TwtxtStatus {
  // -> (full_path, username)
  pub fn get_all_files() -> QueryAs<(String, String)> {
    sqlx::query_as(
      "SELECT full_path, username FROM file
        JOIN user
        ON file.user_id = user.id
        WHERE user_path = 'twtxt.txt'",
    )
  }

  // Will truncate to 280 characters. Spec recommends 140 but that's too short IMO
  pub fn new(username: String, status_text: String) -> Option<Self> {
    let result: Vec<&str> = status_text.splitn(2, '\t').collect();
    if result.len() != 2 {
      return None;
    }
    let mut text = result[1].to_string();
    text.truncate(280);
    if let Ok(datetime) = DateTime::parse_from_rfc3339(result[0]) {
      Some(Self {
        date: datetime.naive_utc(),
        time_ago: rendered_time_ago(datetime.timestamp() as i32),
        username,
        text,
      })
    } else {
      None
    }
  }

  // pub fn text_to_html(&self) -> String {
  // render hyperlinks
  // "".to_string()
  // }
}

#[cfg(test)]
mod tests {
  // Note this useful idiom: importing names from outer (for mod tests) scope.
  use super::*;

  #[test]
  fn test_parse_status() {
    let new_status = TwtxtStatus::new(
      "guy".to_owned(),
      "1996-12-19T16:39:57-08:00\they whats up".to_owned(),
    )
    .unwrap();
    assert_eq!(&new_status.username, "guy");
    assert_eq!(&new_status.text, "hey whats up");
  }

  #[test]
  fn test_invalid_status() {
    let new_status = TwtxtStatus::new(
      "guy".to_owned(),
      "1996-19T16:39:57-08:00\they whats up".to_owned(),
    );
    assert!(new_status.is_none())
  }
}
