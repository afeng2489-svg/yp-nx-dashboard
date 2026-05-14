//! NexusORM — trait-based persistence abstraction for the NexusFlow workspace.
//!
//! ## Why
//!
//! `nx_session::persistence` and `orchestrator::session_store` contain ~200
//! lines of near-identical `rusqlite` boilerplate: `Arc<Mutex<Connection>>`,
//! `execute_batch` schema creation, JSON column serde, and the same
//! `save/get/list/delete` method shapes.
//!
//! This crate provides a shared foundation so new modules don't duplicate
//! the same patterns a third time. Existing code is not forced to migrate.
//!
//! ## Architecture
//!
//! - [`Repository`] — generic CRUD trait (mock-friendly)
//! - [`SqliteStore`] — reusable `rusqlite` wrapper with table DDL, param
//!   binding, and JSON helper methods
//! - [`OrmError`] — unified error type
//!
//! ## Usage
//!
//! ```ignore
//! use nexus_orm::{SqliteStore, Repository, OrmError};
//!
//! let store = SqliteStore::open("app.db")?;
//! store.ensure_table("CREATE TABLE IF NOT EXISTS ...")?;
//! // implement Repository<T, K> for your type using store.conn()
//! ```
//!
//! [`Repository`]: crate::Repository
//! [`SqliteStore`]: crate::SqliteStore
//! [`OrmError`]: crate::OrmError

pub mod error;
pub mod sqlite;
pub mod traits;

pub use error::OrmError;
pub use sqlite::SqliteStore;
pub use traits::Repository;
