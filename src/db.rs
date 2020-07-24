use rusqlite::{Connection, Result};
use rusqlite::NO_PARAMS;

pub fn initialize_tables() -> Result<()> {
    let conn = Connection::open("app.db")?;

    conn.execute(
        "create table if not exists users (
             id integer primary key,
             name text not null unique
             email text not null unique
             password text
         )",
        NO_PARAMS,
    )?;
    conn.execute(
        "create table if not exists cats (
             id integer primary key,
             name text not null,
             color_id integer not null references cat_colors(id)
         )",
        NO_PARAMS,
    )?;

    Ok(())
}
