pub mod edit;
pub mod person;
pub mod sign_up_in_out;

use super::Config;
use crate::records::{File, Person, RenderedFile, TwtxtStatus};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    person: Option<Person>,
    people: Vec<&'a str>,
    files: Vec<RenderedFile<'a>>, // arr?
    config: &'a Config,
}

impl<'a> IndexTemplate<'a> {
    pub fn new(
        person: Option<Person>,
        people: Vec<&'a str>,
        files: Vec<RenderedFile<'a>>,
        config: &'a Config,
    ) -> Self {
        Self {
            person,
            people,
            files,
            config,
        }
    }
}

#[derive(Template)]
#[template(path = "my_site.html")]
pub struct MySiteTemplate<'a> {
    person: Option<Person>,
    files: Vec<File>,
    config: &'a Config,
}

impl<'a> MySiteTemplate<'a> {
    pub fn new(person: Option<Person>, files: Vec<File>, config: &'a Config) -> Self {
        Self {
            person,
            files,
            config,
        }
    }
}

#[derive(Template)]
#[template(path = "statuses.html")]
pub struct StatusesTemplate<'a> {
    person: Option<Person>,
    statuses: Vec<TwtxtStatus>,
    config: &'a Config,
}

impl<'a> StatusesTemplate<'a> {
    pub fn new(person: Option<Person>, statuses: Vec<TwtxtStatus>, config: &'a Config) -> Self {
        Self {
            person,
            statuses,
            config,
        }
    }
}
