/// see https://twtxt.readthedocs.io/en/latest/user/twtxtfile.html
use chrono::{DateTime, NaiveDateTime};
use crate::utils::rendered_time_ago;

pub struct TwtxtStatus {
    pub date: NaiveDateTime,
    pub time_ago: String,
    pub username: String,
    pub text: String, // TODO figure out str
}

impl TwtxtStatus {
    /// Will truncate to 140 characters
    pub fn new(username: String, status_text: String) -> Option<Self> {
        let result: Vec<&str> = status_text.splitn(2, "\t").collect();
        if result.len() != 2 {
            return None;
        }
        let mut text = result[1].to_string();
        text.truncate(140);
        if let Ok(datetime) = DateTime::parse_from_rfc3339(result[0]) {
            return Some(Self {
                date: datetime.naive_utc(),
                time_ago: rendered_time_ago(datetime.timestamp() as u32),
                username: username.to_string(),
                text: text,
            });
        } else {
            return None;
        }
    }

    pub fn text_to_html(&self) -> String {
        // render hyperlinks
        "".to_string()
    }
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
        let new_status = TwtxtStatus::new("guy", "1996-19T16:39:57-08:00\they whats up");
        assert!(new_status.is_none())
    }
}
