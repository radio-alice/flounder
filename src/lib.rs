use crate::twtxt::TwtxtStatus;
use actix_files as fs; // TODO optional
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_multipart::Multipart;
use actix_ratelimit::{MemoryStore, MemoryStoreActor, RateLimiter};
use actix_web::error as actix_error;
use actix_web::middleware::{Logger, NormalizePath};
use actix_web::FromRequest;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use bcrypt;
use env_logger;
use env_logger::Env;
use error::FlounderError;
use futures::{StreamExt, TryStreamExt};
use gmi2html;
use rusqlite::{Connection, Result, NO_PARAMS};
use serde::Deserialize;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::str;
use std::sync::Mutex;
use std::time::Duration;
use utils::*;

mod client;
mod error;
mod templates;
mod twtxt;
mod utils;

use templates::*;

static BASE_INDEX: &[u8] = include_bytes!("baseIndex.gmi");

type DbConn = web::Data<Mutex<Connection>>;

#[derive(Deserialize, Clone)]
struct Config {
    db_path: String,
    file_directory: String,
    tls_enabled: bool,
    server_name: String,
    serve_all_content: bool, // Don't use nginx for anything. In production probably we wanna use nginx for static files
    // Not ready for open registration yet -- use this
    static_path: String,
    proxy_url: String,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

// hacking actix_identity to store user and id
// id, then name
fn parse_identity(id: String) -> (String, String) {
    // TODO fix this shit
    let mut split = id.split_whitespace();
    (
        split.next().unwrap().to_string(),
        split.next().unwrap().to_string(),
    )
}

// TODO user login auth
async fn login(
    id: Identity,
    conn: DbConn,
    form: web::Form<LoginForm>,
) -> Result<HttpResponse, FlounderError> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT id, password_hash from user 
        WHERE user.username = (?)
        "#,
    )?;
    // user does not exist etc
    let (user_id, password_hash): (u32, String) = stmt
        .query_row(&[&form.username], |row| {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        })
        .unwrap_or((0, "notahash".to_string())); // TODO make less awk
    if let Ok(true) = bcrypt::verify(&form.password, &password_hash) {
        // flash?
        id.remember(format!("{} {}", user_id.to_string(), form.username)); // awk
        Ok(HttpResponse::Found()
            .header("Location", "/my_site")
            .finish()) // TODO
    } else {
        // render login page w errors
        let template = LoginTemplate {
            errors: vec!["Invalid username or password!"],
        };
        return template.into_response();
    }
}

async fn logout(id: Identity) -> Result<HttpResponse, Error> {
    id.forget();
    Ok(HttpResponse::Found().header("Location", "/").finish()) // TODO
}

#[derive(Deserialize)]
struct RegisterForm {
    username: String,
    email: String,
    password: String,
    password2: String,
}

impl RegisterForm {
    fn get_errors(&self) -> Vec<&str> {
        let mut errors = vec![];
        if self.username.len() > 32
            || self.username == ""
            || &self.username.to_lowercase() == "www"
            || &self.username.to_lowercase() == "proxy"
        {
            errors.push("Invalid username")
        }
        if !self
            .username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            errors.push("Username must only contain a-z characters and hyphens");
        }
        if !self.email.contains("@") {
            // world's dumbest email verification (we dont really use email)
            errors.push("Email is invalid");
        }
        if self.password.len() < 6 {
            errors.push("Please use a password at least 6 characters long. Preferably longer.");
        }
        if self.password != self.password2 {
            errors.push("Passwords do not match");
        }
        return errors;
    }
}

async fn register(
    id: Identity,
    conn: DbConn,
    form: web::Form<RegisterForm>,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    // validate
    let errors = form.get_errors();
    if errors.len() > 0 {
        return RegisterTemplate {
            errors: errors,
            server_name: &config.server_name,
        }
        .into_response();
    }
    let hashed_pass = bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).unwrap();
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        r#"
        INSERT INTO user (username, email, password_hash)
        VALUES (?1, ?2, ?3)
        "#,
    )?;
    match stmt.execute(&[&form.username.to_lowercase(), &form.email, &hashed_pass]) {
        Ok(_) => (),
        Err(_) => {
            return RegisterTemplate {
                errors: vec!["Username or email already taken"],
                server_name: &config.server_name,
            }
            .into_response()
        }
    }

    let user_id = conn.last_insert_rowid(); // maybe this works

    // todo dont repeat;
    let filename = "index.gmi";
    let full_path = Path::new(&config.file_directory)
        .join(&form.username.to_lowercase())
        .join(filename); // TODO sanitize
    std::fs::create_dir_all(&full_path.parent().unwrap()).ok();
    let mut f = std::fs::File::create(&full_path)?;
    f.write(&BASE_INDEX)?;
    let mut stmt = conn.prepare_cached(
        r#"
    INSERT INTO file (user_path, user_id, full_path)
    VALUES (?1, ?2, ?3)
    "#,
    )?;
    stmt.execute(&[filename, &user_id.to_string(), &full_path.to_str().unwrap()])?;

    id.remember(format!(
        "{} {}",
        user_id.to_string(),
        form.username.to_lowercase()
    )); // awk
        // redirect to my site
    Ok(HttpResponse::Found()
        .header("Location", "/edit/index.gmi")
        .finish())
}

async fn index(
    id: Identity,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let conn = conn.lock().unwrap(); // TODO
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT user.username from user;
        "#,
    )?;
    let mut usernames = vec![];
    let mut users_res = stmt.query(NO_PARAMS)?;
    while let Some(row) = users_res.next()? {
        usernames.push(row.get(0)?);
    }

    let mut stmt = conn.prepare_cached(
        r#"
        SELECT user.username, file.user_path, file.updated_at 
        FROM file 
        JOIN user
        ON file.user_id = user.id
        ORDER BY file.updated_at DESC
        LIMIT 100"#,
    )?;
    let files_res = stmt.query_map(NO_PARAMS, |row| {
        Ok(RenderedFile {
            username: row.get(0)?,
            user_path: row.get(1)?,
            time_ago: rendered_time_ago(row.get(2)?),
        })
    })?;
    let template = IndexTemplate {
        logged_in: id.identity().is_some(),
        server_name: &config.server_name,
        files: files_res.map(|a| a.unwrap()).collect(),
        users: usernames,
    };
    template.into_response()
}

async fn register_page(config: web::Data<Config>) -> Result<HttpResponse, FlounderError> {
    RegisterTemplate {
        errors: vec![],
        server_name: &config.server_name,
    }
    .into_response()
}

async fn login_page() -> Result<HttpResponse, FlounderError> {
    LoginTemplate { errors: vec![] }.into_response()
}

async fn my_site(
    id: Identity,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    // replace impl with specific
    if let Some(idstr) = id.identity() {
        let (user_id, username) = parse_identity(idstr);
        let conn = conn
            .lock()
            .map_err(|_| actix_error::ErrorInternalServerError("Internal Server Error"))?;
        let mut stmt = conn
            .prepare_cached(
                r#"
            SELECT file.user_path, file.updated_at
            FROM file where user_id = (?)
            "#,
            )
            .unwrap();
        let res = stmt
            .query_map(&[user_id], |row| {
                Ok(RenderedFile {
                    username: username.clone(), // TODO remove clone
                    user_path: row.get(0)?,
                    time_ago: rendered_time_ago(row.get(1)?),
                })
            })
            .map_err(actix_error::ErrorInternalServerError)?;
        MySiteTemplate {
            logged_in: true,
            username: &username,
            errors: vec![],
            server_name: &config.server_name,
            files: res.map(|a| a.unwrap()).collect(),
        }
        .into_response()
    } else {
        // flash you must be logged in?
        Ok(HttpResponse::Found().header("Location", "/login").finish()) // TODO
    }
}

#[derive(Deserialize)]
struct EditFileForm {
    file_text: String,
}

async fn edit_file_page(
    id: Identity,
    local_path: web::Path<String>,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    // read file to string
    let identity = id
        .identity()
        .ok_or(error::FlounderError::UnauthorizedError)?;
    let (_, username) = parse_identity(identity);
    let filename = sanitize_filename::sanitize(local_path.as_str());
    let full_path = Path::new(&config.file_directory)
        .join(&username)
        .join(&filename); // TODO sanitize
    let file_text = std::fs::read_to_string(full_path).unwrap_or("".to_string());
    let template = EditFileTemplate {
        filename: filename,
        file_text: file_text,
    };
    return template.into_response();
}

// return error strs
// this function is weird because i'm bad at rust
fn upsert_file(
    data: &[u8],
    conn: &DbConn,
    username: &str,
    user_id: &str,
    local_path: &str,
    file_directory: &str,
) -> Result<Vec<String>, FlounderError> {
    let mut errors = vec![];
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT COUNT(*) FROM file
        where user_id = (?1)
        "#,
    )?;
    let count: u32 = stmt.query_row(&[user_id], |r| r.get(0))?;
    if count >= 128 {
        return Ok(vec!["You have the max number of files. Delete some to make room for more.".to_owned()]);
    }
    let filename = &sanitize_filename::sanitize(local_path);
    // validate
    if !ok_extension(filename) {
        errors.push("Invalid file extension.".to_owned());
    }
    let full_path = Path::new(&file_directory).join(&username).join(filename);
    std::fs::create_dir_all(full_path.parent().unwrap()).ok();
    if errors.len() > 0 {
        return Ok(errors);
    }
    let mut stmt = conn.prepare_cached(
        r#"
    INSERT INTO file (user_path, user_id, full_path)
    VALUES (?1, ?2, ?3)
    ON CONFLICT(full_path) DO UPDATE SET
    updated_at=strftime('%s', 'now')
    "#,
    )?;
    // TODO -- limit max files per user to configurable X (default 128)
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&full_path)?;
    file.write(data)?;
    stmt.execute(&[filename, user_id, full_path.to_str().unwrap()])?;
    Ok(vec![])
}

async fn edit_file(
    id: Identity,
    form: web::Form<EditFileForm>,
    local_path: web::Path<String>,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let identity = id
        .identity()
        .ok_or(error::FlounderError::UnauthorizedError)?;
    let file_directory: String = config.file_directory.clone();
    let (user_id, username) = parse_identity(identity);
    let errors = upsert_file(
        form.file_text.as_bytes(),
        &conn,
        &username,
        &user_id,
        local_path.as_str(),
        &file_directory,
    )?;
    if errors.len() > 0 {
        // temporary
        return Ok(HttpResponse::InternalServerError().body(format!("{:?}", errors)));
    }
    Ok(HttpResponse::Found()
        .header("Location", "/my_site")
        .finish()) // TODO g
}

/// Overwrites existing files
/// copied from update_file a lot. TODO merge
async fn upload_file(
    id: Identity,
    mut payload: Multipart,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let identity = id
        .identity()
        .ok_or(error::FlounderError::UnauthorizedError)?;
    let (user_id, username) = parse_identity(identity); // fail otheriwse
    let file_directory: String = config.file_directory.clone();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        let mut all_data = vec![];
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            all_data.extend(data);
        }
        let errors = upsert_file(
            &all_data,
            &conn,
            &username,
            &user_id,
            filename,
            &file_directory,
        )?;
        if errors.len() > 0 {
            // temporary
            return Ok(HttpResponse::InternalServerError().body(format!("{:?}", errors)));
        }

        // TODO error handling
    }
    Ok(HttpResponse::Found()
        .header("Location", "/my_site")
        .finish()) // TODO g
}

async fn delete_file(
    conn: DbConn,
    id: Identity,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let identity = id
        .identity()
        .ok_or(error::FlounderError::UnauthorizedError)?;
    let (_, username) = parse_identity(identity); // fail otheriwse
    let conn = conn.lock().unwrap();
    let filename = &sanitize_filename::sanitize(path.as_str());
    let full_path = Path::new(&config.file_directory)
        .join(&username)
        .join(filename); // TODO sanitize
    std::fs::remove_file(&full_path).ok();

    let mut stmt = conn.prepare_cached(
        r#"
    DELETE FROM file where file.full_path = (?)
    "#,
    )?;
    stmt.execute(&[&full_path.to_str()])?;
    // verify idetntiy
    // remove file from dir
    // delete from db
    Ok(HttpResponse::Found()
        .header("Location", "/my_site")
        .finish()) // TODO g
}

// redundant -- cleanup
async fn serve_home(
    user: web::Path<String>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let full_path = Path::new(&config.file_directory)
        .join(user.as_str())
        .join("index.gmi");
    let gmi_file = std::fs::read_to_string(full_path).unwrap();
    let string = gmi2html::GeminiConverter::new(&gmi_file)
        .proxy_url(&config.proxy_url)
        .inline_images(true)
        .to_html();
    let template = GmiPageTemplate {
        title: user.as_str(),
        html_block: &string,
    };
    return Ok(template.into_response().unwrap());
}
/// Rather than route through the gmi server, we write an
/// HTTP client that behaves like the gmi proxy, for performance
/// replace some w/ nginx?
async fn serve_user_content(
    path: web::Path<(String, String)>,
    r: HttpRequest,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let username = &path.0;
    let filename = &sanitize_filename::sanitize(&path.1); // probably not necc but eh/
    let full_path = Path::new(&config.file_directory)
        .join(&username)
        .join(filename);
    // empty path render index
    if full_path.extension() == Some(OsStr::new("gmi"))
        || full_path.extension() == Some(OsStr::new("gemini"))
    {
        let gmi_file = std::fs::read_to_string(full_path).unwrap();
        if r.query_string() == "raw=1" {
            return Ok(HttpResponse::from(gmi_file));
        }
        let string = gmi2html::GeminiConverter::new(&gmi_file)
            .proxy_url(&config.proxy_url)
            .inline_images(true)
            .to_html();
        let template = GmiPageTemplate {
            title: filename,
            html_block: &string,
        };
        return Ok(template.into_response().unwrap());
    }
    fs::NamedFile::open(full_path).unwrap().into_response(&r) // todo error
}

async fn proxy(url: web::Path<String>) {
    client::get_gmi_data(&url);
}

async fn show_statuses(
    id: Identity,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        r#"
    SELECT full_path, username FROM file
    JOIN user 
    ON file.user_id = user.id
    WHERE user_path = 'twtxt.txt'"#,
    )?;

    let mut statuses: Vec<TwtxtStatus> = vec![];
    let mut res = stmt.query(NO_PARAMS)?;
    while let Some(row) = res.next()? {
        let full_path: String = row.get(0).unwrap();
        let status_data = std::fs::read_to_string(full_path).unwrap();
        for line in status_data.lines() {
            let new_status = TwtxtStatus::new(row.get(1).unwrap(), line.to_string());
            if new_status.is_some() {
                statuses.push(new_status.unwrap());
            }
        }
    }
    statuses.sort_unstable_by_key(|a| a.date);
    statuses.reverse();
    // get all statuses push to vec
    // sort statuses by date
    let template = StatusesTemplate {
        logged_in: id.identity().is_some(),
        statuses: statuses,
        server_name: &config.server_name,
    };
    template.into_response()
}
// https://actix.rs/docs/extractors/
// run gemini server in separate thread
#[actix_rt::main]
pub async fn run_server(config_path: String) -> std::io::Result<()> {
    // Error type?
    // db::initialize_tables().unwrap();
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    // parse arguments using light library
    // initialize config
    HttpServer::new(move || {
        let config_str = std::fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&config_str).unwrap();
        let store = MemoryStore::new(); // used for ratelimit
        let conn = Mutex::new(Connection::open(&config.db_path).unwrap()); // TODO config, error?
        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath) // does this do anything
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    // domain?
                    // https://docs.rs/actix-identity/0.3.0-alpha.1/actix_identity/struct.CookieIdentityPolicy.html
                    .name("auth-cookie")
                    .secure(false),
            ))
            .data(conn)
            .app_data(web::Form::<EditFileForm>::configure(|cfg| {
                cfg.limit(32 * 1024)
            }))
            .service(fs::Files::new("/static", &config.static_path).show_files_listing()) // TODO configurable
            .data(config)
            .route("/", web::get().to(index))
            // TODO -- setup to use nginx in production
            .route("/my_site", web::get().to(my_site))
            .service(
                web::resource("/login")
                    .route(web::post().to(login)) // TODO figure out how to just rate limit one of this
                    .route(web::get().to(login_page))
                    .wrap(
                        RateLimiter::new(MemoryStoreActor::from(store.clone()).start())
                            .with_interval(Duration::from_secs(60))
                            .with_max_requests(20),
                    ), //   DO consolidate
            )
            .route("/logout", web::get().to(logout)) // TODO should be post
            .service(
                web::resource("/register")
                    .route(web::post().to(register))
                    .route(web::get().to(register_page)) // TODO better rate limiting
                    .wrap(
                        RateLimiter::new(MemoryStoreActor::from(store.clone()).start())
                            .with_interval(Duration::from_secs(86400))
                            .with_max_requests(20),
                    ),
            )
            .route("/register", web::get().to(register_page))
            .route("/statuses", web::get().to(show_statuses))
            .route("/upload", web::post().to(upload_file))
            .route(
                "/user/{username}/{user_file_path}",
                web::get().to(serve_user_content),
            )
            .route("/user/{username}/", web::get().to(serve_home))
            .route("/edit/{user_file_path}", web::get().to(edit_file_page))
            .route("/edit/{user_file_path}", web::post().to(edit_file))
            .route("/delete/{user_file_path}", web::post().to(delete_file))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
