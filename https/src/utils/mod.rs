use crate::records::Person;
use std::result::Result;
use std::time::SystemTime;

pub mod escape;
pub mod gemtext2html;

pub(crate) async fn deserialize_body<T>(request: &mut crate::Request) -> tide::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    match request.content_type() {
        Some(c) if c == tide::http::mime::FORM => request.body_form().await,
        Some(c) if c == tide::http::mime::JSON => request.body_json().await,
        _ => Err(tide::Error::from_str(
            tide::StatusCode::NotAcceptable,
            "unrecognized content type",
        )),
    }
}

/// use request.get_person() to get logged in person IFF it is required that they be logged in
pub trait GetPerson {
    fn get_person(&self) -> Result<Person, tide::Error>;
}
impl GetPerson for crate::Request {
    fn get_person(&self) -> Result<Person, tide::Error> {
        self.session().get("person").ok_or_else(|| {
            tide::Error::from_str(tide::StatusCode::Unauthorized, "yo you're not logged in")
        })
    }
}

pub trait AsRoute {
    fn as_route(&self) -> std::borrow::Cow<str>;
}

impl AsRoute for str {
    fn as_route(&self) -> std::borrow::Cow<str> {
        self.into()
    }
}

impl AsRoute for String {
    fn as_route(&self) -> std::borrow::Cow<str> {
        self.into()
    }
}

// pub fn redirect_to(record: impl AsRoute) -> tide::Response {
//     tide::Redirect::new(record.as_route()).into()
// }

pub fn rendered_time_ago(epoch_time: i32) -> String {
    // do some fun stuff
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ago = now - epoch_time as u64;

    if ago < 60 {
        return format!("{} seconds ago", ago);
    } else if ago < 3600 {
        return format!("{} minutes ago", ago / 60);
    } else if ago < 3600 * 24 {
        return format!("{} hours ago", ago / 3600);
    } else {
        return format!("{} days ago", ago / (3600 * 24));
    }
}
