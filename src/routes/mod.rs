pub mod file;
pub mod person;
pub mod sign_up_in_out;

use super::{Config, State};
use crate::records::{File, Person, RenderedFile, TwtxtStatus};
use crate::templates::{IndexTemplate, MySiteTemplate, StatusesTemplate};
use crate::utils::{deserialize_body, rendered_time_ago, GetPerson};
use serde::Deserialize;
use sqlx::prelude::*;
use std::io::Write;
use tide::{Error, StatusCode};

pub async fn people(_request: crate::Request) -> tide::Result {
    Ok(tide::Redirect::new("/").into())
}

pub async fn my_site(request: crate::Request) -> tide::Result {
    let State { db, config } = request.state();
    if let Ok(person) = request.get_person() {
        let files = File::all_for_person(person.id).fetch_all(db).await?;
        Ok(MySiteTemplate::new(Some(person), files, &config).into())
    } else {
        Ok(tide::Redirect::new("/sign/in").into())
    }
}

pub async fn index(request: crate::Request) -> tide::Result {
    let State { db, config } = request.state();
    let person = request.session().get("person");
    let people_raw = Person::all_names().fetch_all(db).await?;
    let people = people_raw.iter().map(|(person,)| &person[..]).collect();
    let files_raw = File::get_recent().fetch_all(db).await?;
    let files = files_raw
        .iter()
        .map(|file| RenderedFile {
            username: &file.username,
            user_path: &file.user_path,
            time_ago: rendered_time_ago(file.updated_at),
        })
        .collect();
    Ok(IndexTemplate::new(person, people, files, &config).into())
}

pub async fn statuses_get(request: crate::Request) -> tide::Result {
    let State { db, config } = request.state();
    let person = request.session().get("person");
    let status_files = TwtxtStatus::get_all_files().fetch_all(db).await?;
    let mut statuses: Vec<TwtxtStatus> = vec![];
    for (full_path, username) in status_files {
        let status_file = std::fs::read_to_string(full_path).unwrap_or_else(|_| "".into());
        let status_lines = status_file.lines();
        for line in status_lines {
            if let Some(new_status) = TwtxtStatus::new(username.clone(), line.to_string()) {
                statuses.push(new_status);
            }
        }
    }
    statuses.sort_unstable_by_key(|a| a.date);
    statuses.reverse();
    Ok(StatusesTemplate::new(person, statuses, &config).into())
}

#[derive(Deserialize)]
struct AppendStatusForm {
    status_text: String,
}
pub async fn statuses_post(mut request: crate::Request) -> tide::Result {
    let State { config, db } = request.state().clone();
    let person = request.get_person()?;
    let twtxt_path = File::get_full_path(&config.file_directory, &person.username, "twtxt.txt");
    let mut twtxt_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&twtxt_path)?;
    let form = deserialize_body::<AppendStatusForm>(&mut request).await?;
    match twtxt_file.write_all(form.status_text.as_bytes()) {
        Ok(_) => {
            File::upsert(
                "twtxt.txt",
                person.id,
                twtxt_path.to_str().ok_or_else(|| {
                    Error::from_str(
                        StatusCode::InternalServerError,
                        "couldn't convert path to string",
                    )
                })?,
            )
            .execute(&db)
            .await?;
            Ok(tide::Response::from(StatusCode::Ok))
        }
        Err(_) => Ok(Error::from_str(
            StatusCode::InternalServerError,
            "failed to write to twtxt file",
        )
        .into()),
    }
}
