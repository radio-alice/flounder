CREATE TABLE user (
    id INTEGER NOT NULL, 
    username TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT,
    created_at INTEGER, 
    updated_at INTEGER, 
    PRIMARY KEY (id)
);
CREATE TABLE file (
    id INTEGER NOT NULL, 
    full_path TEXT UNIQUE,
    user_path TEXT,
    user_id INTEGER, 
    created_at INTEGER, 
    updated_at INTEGER, 
    PRIMARY KEY (id), 
    FOREIGN KEY(user_id) REFERENCES user (id), 
);
