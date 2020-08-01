CREATE TABLE user (
    id INTEGER NOT NULL, 
    username TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT,
    created_at INTEGER  DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (id)
);
CREATE TABLE file (
    id INTEGER NOT NULL, 
    user_path TEXT,
    full_path TEXT UNIQUE,
    user_id INTEGER, 
    created_at INTEGER  DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (id), 
    FOREIGN KEY(user_id) REFERENCES user (id)
);
