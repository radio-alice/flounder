use anyhow::{anyhow, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::{prelude::SqliteQueryAs, sqlite::SqlitePool};
use tide::{Error, StatusCode};

use super::{Config, State};
use crate::records::{File, NewPerson, Person};
use crate::templates::sign_up_in_out::*;
use crate::utils;

#[derive(Deserialize)]
struct LoginForm {
  username: String,
  password: String,
}

#[derive(Template)]
#[template(path = "baseIndex.txt")]
struct BaseGmiTemplate<'a> {
  config: &'a Config,
}
impl<'a> BaseGmiTemplate<'a> {
  pub fn new(config: &'a Config) -> Self {
    Self { config }
  }
}

impl LoginForm {
  pub async fn validate(&self, db: &SqlitePool) -> Result<()> {
    let person = &Person::find_by_username(&self.username)
      .fetch_one(db)
      .await?;

    if bcrypt::verify(&self.password, &person.password_hash)? {
      Ok(())
    } else {
      Err(anyhow!("wrong password"))
    }
  }
}

pub async fn get_in(request: crate::Request) -> tide::Result {
  let State { config, db: _ } = request.state();
  Ok(InTemplate::new(&config, None).into())
}

pub async fn post_in(mut request: crate::Request) -> tide::Result {
  let State { db, config } = &request.state().clone();
  let login: LoginForm = utils::deserialize_body(&mut request).await?;
  let res = login.validate(db).await;
  match res {
    Ok(_) => {
      let person = Person::find_by_username(&login.username)
        .fetch_one(db)
        .await?;
      request.session_mut().insert("person", person)?;
      Ok(tide::Redirect::new("/my_site").into())
    }
    Err(e) => {
      let error = &e.to_string()[..];
      Ok(InTemplate::new(&config, Some(error)).into())
    }
  }
}

pub async fn get_out(mut request: crate::Request) -> tide::Result {
  request.session_mut().remove("person");
  Ok(OutTemplate::new().into())
}

pub async fn get_up(request: crate::Request) -> tide::Result {
  let State { config, db: _ } = request.state();
  Ok(UpTemplate::new(&config, &[]).into())
}

pub async fn post_up(mut request: crate::Request) -> tide::Result {
  let State { db, config } = &request.state().clone();
  let new_person: NewPerson = utils::deserialize_body(&mut request).await?;
  let errors = new_person.validate();
  if !errors.is_empty() {
    Ok(UpTemplate::new(&config, errors.as_slice()).into())
  } else {
    let (person_id,) = new_person.create().fetch_one(db).await?;
    let person = Person::find_by_id(person_id)
      .fetch_one(db)
      .await
      .map_err(|_| Error::from_str(StatusCode::Conflict, "that username or email is taken"))?;

    // create person's directory
    person
      .create_dir(&config.file_directory)
      .map_err(|e| Error::from_str(StatusCode::InternalServerError, e))?;

    // create index.gmi
    // yes we're rendering it on every signup, but the alternative is some really gross code and it seems unlikely that we'd have so many signups that this is a real performance issue
    let full_path = File::get_full_path(&config.file_directory, &person.username, "index.gmi");
    let index_gmi = BaseGmiTemplate::new(&config).render()?;
    let path_str = full_path.to_str().ok_or_else(|| {
      Error::from_str(
        StatusCode::InternalServerError,
        "couldn't convert path to string",
      )
    })?;
    File::upsert("index.gmi", person.id, path_str)
      .execute(db)
      .await?;
    File::write_all(index_gmi.as_bytes(), full_path)?;
    request.session_mut().insert("person", person)?;
    Ok(tide::Redirect::new("/my_site").into())
  }
}
