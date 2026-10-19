//! SQLite-backed storage for the todo app (local store for v1).

pub mod migrations;
mod sqlite_repo;

pub use sqlite_repo::SqliteRepo;
