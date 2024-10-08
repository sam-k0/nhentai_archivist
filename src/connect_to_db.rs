// Copyright (c) 2024 구FS, all rights reserved. Subject to the MIT licence in `licence.md`.
use sqlx::ConnectOptions;
use sqlx::migrate::MigrateDatabase;


/// # Summary
/// Connects to database at `database_url` and returns a connection pool. If database does not exist, creates a new database and initialises it with the instructions in `./db/create_db.sql`.
///
/// # Arguments
/// - `database_url`: path to database file
///
/// # Returns
/// - connection pool to database or error
pub async fn connect_to_db(database_url: &str) -> Result<sqlx::sqlite::SqlitePool, sqlx::Error>
{
    const CREATE_DB_QUERY_STRING: &str = // query string to create all tables except the dynamically created Hentai_{id}_Pages
        "CREATE TABLE Hentai
        (
            id INTEGER NOT NULL,
            cover_type TEXT NOT NULL,
            media_id INTEGER NOT NULL,
            num_favorites INTEGER NOT NULL,
            num_pages INTEGER NOT NULL,
            page_types TEXT NOT NULL,
            scanlator TEXT,
            title_english TEXT,
            title_japanese TEXT,
            title_pretty TEXT,
            upload_date TEXT NOT NULL,
            PRIMARY KEY(id)
        );
        CREATE TABLE Tag
        (
            id INTEGER NOT NULL,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            url TEXT NOT NULL,
            PRIMARY KEY(id)
        );
        CREATE TABLE Hentai_Tag
        (
            hentai_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY(hentai_id, tag_id),
            FOREIGN KEY(hentai_id) REFERENCES Hentai(id),
            FOREIGN KEY(tag_id) REFERENCES Tag(id)
        );";
    let db: sqlx::sqlite::SqlitePool; // database containing all metadata from nhentai.net api


    if !sqlx::sqlite::Sqlite::database_exists(database_url).await? // if database does not exist
    {
        match std::path::Path::new(database_url).parent()
        {
            Some(parent) =>
            {
                #[cfg(target_family = "unix")]
                if let Err(e) = tokio::fs::DirBuilder::new().recursive(true).mode(0o777).create(parent).await // create all parent directories with permissions "drwxrwxrwx"
                {
                    log::warn!("Creating parent directories for new database at \"{database_url}\" failed with {e}.\nThis could be expected behaviour, usually if this is a remote pointing URL and not a local filepath. In that case create the parent directories manually.");
                }
                #[cfg(not(target_family = "unix"))]
                if let Err(e) = tokio::fs::DirBuilder::new().recursive(true).create(parent).await // create all parent directories
                {
                    log::warn!("Creating parent directories for new database at \"{database_url}\" failed with {e}.\nThis could be expected behaviour, usually if this is a remote pointing URL and not a local filepath. In that case create the parent directories manually.");
                }
            }
            None => log::warn!("Creating parent directories for new database at \"{database_url}\", because the directory part could not be parsed.\nThis could be expected behaviour, usually if this is a remote pointing URL and not a local filepath. In that case create the parent directories manually."),
        }
        sqlx::sqlite::Sqlite::create_database(database_url).await?; // create new database
        log::info!("Created new database at \"{}\".", database_url);

        db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1) // only 1 connection to database at the same time, otherwise concurrent writers fail
            .max_lifetime(None) // keep connection open indefinitely otherwise database locks up after lifetime, closing and reconnecting manually
            .connect(database_url).await?; // connect to database
        db.set_connect_options(sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // use write-ahead journal for better performance
            .locking_mode(sqlx::sqlite::SqliteLockingMode::Exclusive) // do not release file lock until all transactions are complete
            .log_slow_statements(log::LevelFilter::Warn, std::time::Duration::from_secs(5)) // log slow statements only after 5 s
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)); // ensure data is written to disk after each transaction for consistent state
        log::info!("Connected to database at \"{}\".", database_url);

        sqlx::query(CREATE_DB_QUERY_STRING).execute(&db).await?; // initialise database by creating tables
        log::info!("Created database tables.");
    }
    else // if database already exists
    {
        db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1) // only 1 connection to database at the same time, otherwise concurrent writers fail
            .max_lifetime(None) // keep connection open indefinitely otherwise database locks up after lifetime, closing and reconnecting manually
            .connect(database_url).await?; // connect to database
        db.set_connect_options(sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // use write-ahead journal for better performance
            .locking_mode(sqlx::sqlite::SqliteLockingMode::Exclusive) // do not release file lock until all transactions are complete
            .log_slow_statements(log::LevelFilter::Warn, std::time::Duration::from_secs(5)) // log slow statements only after 5 s
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)); // ensure data is written to disk after each transaction for consistent state
        log::info!("Connected to database at \"{}\".", database_url);
    }

    return Ok(db);
}