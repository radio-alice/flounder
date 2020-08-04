use actix_files as fs; // TODO optional
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_multipart::Multipart;
use actix_web::error as actix_error;
use actix_web::middleware::{Logger, NormalizePath};
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
use utils::rendered_time_ago;

mod error;
mod templates;
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
    secret_key: String,
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
    let (user_id, password_hash): (u32, String) = stmt.query_row(&[&form.username], |row| {
        Ok((row.get(0).unwrap(), row.get(1).unwrap()))
    })?;
    if bcrypt::verify(&form.password, &password_hash).unwrap() {
        // flash?
        id.remember(format!("{} {}", user_id.to_string(), form.username)); // awk
        Ok(HttpResponse::Found()
            .header("Location", "/my_site")
            .finish()) // TODO
    } else {
        Ok(HttpResponse::Found().header("Location", "/login").finish())
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
    secret: String,
}

impl RegisterForm {
    fn validate(&self, secret_key: &str) -> bool {
        // username must be letters numbers hyphens
        if !(self.secret == secret_key) {
            // for debug
            return false;
        }
        if !self
            .username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return false;
        }
        if !self.email.contains("@") {
            // world's dumbest email verification (we dont really use email)
            return false;
        }
        if self.password.len() < 6 {
            return false;
        }
        return self.password == self.password2;
    }
}

async fn register(
    id: Identity,
    conn: DbConn,
    form: web::Form<RegisterForm>,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    // validate
    if !form.validate(&config.secret_key) {
        // flash errors
        return Ok(HttpResponse::Found()
            .header("Location", "/register")
            .finish()); // TODO g
    }
    let hashed_pass = bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).unwrap();
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        r#"
        INSERT INTO user (username, email, password_hash)
        VALUES (?1, ?2, ?3)
        "#,
    )?;
    stmt.execute(&[&form.username.to_lowercase(), &form.email, &hashed_pass])?;

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
    Ok(HttpResponse::Found().header("Location", "/").finish())
}

async fn index(
    id: Identity,
    conn: DbConn,
    config: web::Data<Config>,
) -> Result<HttpResponse, FlounderError> {
    let conn = conn.lock().unwrap(); // TODO
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT user.username, file.user_path, file.updated_at 
        FROM file 
        JOIN user
        ON file.user_id = user.id
        ORDER BY file.updated_at DESC
        LIMIT 100"#,
    )?;
    let res = stmt.query_map(NO_PARAMS, |row| {
        Ok(RenderedFile {
            username: row.get(0)?,
            user_path: row.get(1)?,
            time_ago: rendered_time_ago(row.get(2)?),
        })
    })?;
    let template = IndexTemplate {
        logged_in: id.identity().is_some(),
        server_name: &config.server_name,
        files: res.map(|a| a.unwrap()).collect(),
    };
    template.into_response()
}

async fn register_page() -> Result<HttpResponse, FlounderError> {
    Ok(RegisterTemplate {}.into_response().unwrap())
}

async fn login_page() -> Result<HttpResponse, FlounderError> {
    LoginTemplate {}.into_response()
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
    let (user_id, username) = parse_identity(identity);
    let conn = conn.lock().unwrap();
    let filename = &sanitize_filename::sanitize(local_path.as_str());
    let full_path = Path::new(&config.file_directory)
        .join(&username)
        .join(filename);
    std::fs::create_dir_all(full_path.parent().unwrap()).ok();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&full_path)?;
    file.write(form.file_text.as_bytes())?;
    let mut stmt = conn.prepare_cached(
        r#"
        INSERT INTO file (user_path, user_id, full_path)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(full_path) DO UPDATE SET
        updated_at=strftime('%s', 'now')
    "#,
    )?;
    stmt.execute(&[filename, &user_id, full_path.to_str().unwrap()])?;

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
    let conn = conn.lock().unwrap();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = &sanitize_filename::sanitize(content_type.get_filename().unwrap());
        let full_path = Path::new(&config.file_directory)
            .join(&username)
            .join(filename); // TODO sanitize
                             // File::create is blocking operation, use threadpool
        let mut f = web::block(move || {
            // create dirs if dne
            std::fs::create_dir_all(full_path.parent().unwrap()).ok();
            std::fs::File::create(full_path)
        })
        .await
        .unwrap();
        let full_path = Path::new(&config.file_directory)
            .join(&username)
            .join(filename); // TODO sanitize
                             // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f))
                .await
                .unwrap();
        }
        let mut stmt = conn.prepare_cached(
            r#"
        INSERT INTO file (user_path, user_id, full_path)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(full_path) DO UPDATE SET
        updated_at=strftime('%s', 'now')
        "#,
        )?;
        stmt.execute(&[filename, &user_id, full_path.to_str().unwrap()])?;

        // TODO work on security
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
    web::block(move || {
        // create dirs if dne
        std::fs::remove_file(full_path)
    })
    .await
    .ok();

    let mut stmt = conn.prepare_cached(
        r#"
    DELETE FROM file where file.user_path = (?)
    "#,
    )?;
    stmt.execute(&[&filename])?;
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
        .to_html();
    let template = GmiPageTemplate { html_block: string };
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
        let string = gmi2html::GeminiConverter::new(&gmi_file)
            .proxy_url(&config.proxy_url)
            .to_html();
        let template = GmiPageTemplate { html_block: string };
        return Ok(template.into_response().unwrap());
    }
    fs::NamedFile::open(full_path).unwrap().into_response(&r) // todo error
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
        let conn = Mutex::new(Connection::open(&config.db_path).unwrap()); // TODO config, error?
        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath)
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    // domain?
                    // https://docs.rs/actix-identity/0.3.0-alpha.1/actix_identity/struct.CookieIdentityPolicy.html
                    .name("auth-cookie")
                    .secure(false),
            ))
            .data(conn)
            .service(fs::Files::new("/static", &config.static_path).show_files_listing()) // TODO configurable
            .data(config)
            .app_data(web::PayloadConfig::new(32000))
            .route("/", web::get().to(index))
            // TODO -- setup to use nginx in production
            .route("/my_site", web::get().to(my_site))
            .route("/login", web::post().to(login)) // TODO consolidate
            .route("/login", web::get().to(login_page))
            .route("/logout", web::get().to(logout)) // TODO should be post
            .route("/register", web::post().to(register))
            .route("/register", web::get().to(register_page))
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
