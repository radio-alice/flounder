use argh::FromArgs;
use async_sqlx_session::SqliteSessionStore;
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;
use std::path::Path;
use std::time::Duration;
use tide::sessions::SessionMiddleware;

mod records;
mod routes;
mod templates;
mod utils;

#[derive(FromArgs, PartialEq, Debug)]
/// A command with positional arguments.
struct Arguments {
    #[argh(subcommand)]
    sub: Sub,
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "admin")]
struct Admin {
    #[argh(option)]
    /// not implemented yet
    x: usize,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Run server
#[argh(subcommand, name = "run")]
struct RunServer {
    /// config file path
    #[argh(option, short = 'c', default = "\"flounder.toml\".to_string()")]
    config: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Sub {
    Admin(Admin),
    RunServer(RunServer),
}

#[derive(Deserialize, Clone)]
pub struct Config {
    tide_secret: String,
    db_path: String,
    root_domain: String,
    server_name: String,
    file_directory: String,
    static_path: String,
}

#[derive(Clone)]
pub struct State {
    db: SqlitePool,
    config: Config,
}

pub type Request = tide::Request<State>;

async fn db_connection(db_url: &str) -> tide::Result<SqlitePool> {
    Ok(SqlitePool::new(db_url).await?)
}

async fn build_session_middleware(
    db: SqlitePool,
    secret: &str,
) -> tide::Result<SessionMiddleware<SqliteSessionStore>> {
    let session_store = SqliteSessionStore::from_client(db);
    session_store.migrate().await?;
    session_store.spawn_cleanup_task(Duration::from_secs(60 * 15));
    Ok(SessionMiddleware::new(session_store, secret.as_bytes()))
}

async fn run_server(config_file: &str) -> tide::Result<()> {
    tide::log::with_level(tide::log::LevelFilter::Info);
    let config_str = std::fs::read_to_string(config_file)?;
    let config: Config = toml::from_str(&config_str)?;
    let db = db_connection(&config.db_path).await?;

    // check if file_dir exists and panic if it doesn't & we can't create it
    let file_dir_exists = Path::new(&config.file_directory).exists();
    if !file_dir_exists {
        std::fs::create_dir_all(&config.file_directory)
        .expect("file directory from config doesn't exist and we don't have the permissions to create it!");
        println!("created file directory\n")
    }

    let mut app = tide::with_state(State {
        db: db.clone(),
        config: config.clone(),
    });

    app.with(build_session_middleware(db, &config.tide_secret).await?);

    // TODO use middleware to limit body content length based on config
    // see https://github.com/http-rs/tide/issues/448

    app.at("/").get(routes::index);
    app.at("/static").serve_dir(&config.static_path)?;
    let mut person = app.at("/person");
    person.at("/").get(routes::people);
    // dumb but necessary
    person.at("/:person").get(routes::person::get);
    person.at("/:person/").get(routes::person::get);
    person.at("/:person/:file_name").get(routes::person::get);

    app.at("/sign/up")
        .get(routes::sign_up_in_out::get_up)
        .post(routes::sign_up_in_out::post_up);

    app.at("/sign/in")
        .get(routes::sign_up_in_out::get_in)
        .post(routes::sign_up_in_out::post_in);
    app.at("/sign/out").get(routes::sign_up_in_out::get_out);

    app.at("/my_site").get(routes::my_site);
    app.at("/statuses")
        .get(routes::statuses_get)
        .post(routes::statuses_post);

    app.at("/upload").post(routes::file::post_upload);
    app.at("/delete/:file_name").post(routes::file::delete);
    app.at("/edit/:file_name")
        .get(routes::file::get_edit)
        .post(routes::file::post_edit);

    app.listen("127.0.0.1:8000").await?;
    Ok(())
}

#[async_std::main]
async fn main() {
    let arg: Arguments = argh::from_env();
    match arg.sub {
        Sub::RunServer(r) => run_server(&r.config).await,
        _ => Ok(()),
    }
    .unwrap();
}
