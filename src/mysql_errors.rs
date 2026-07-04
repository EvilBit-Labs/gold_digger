//! Named constants for MySQL/MariaDB server error codes referenced in
//! query-error classification.
//!
//! `src/run.rs::map_query_error` and the TLS connection classifier match
//! these codes when translating an opaque [`mysql::Error`] into an
//! operator-facing message. Using named constants instead of bare numeric
//! literals makes the call sites grep-able (e.g. `rg ER_PARSE_ERROR`)
//! and keeps the mapping documented next to the canonical reference
//! (todo #054).
//!
//! Values come directly from the MySQL/MariaDB error reference and the
//! mysql client library:
//!   - `ER_*` (1000-series): server-side error codes returned by the
//!     server for SQL parse / privilege / object-existence failures.
//!   - `CR_*` (2000-series): client-side error codes from the C client
//!     library, surfaced by the Rust `mysql` crate when the connection
//!     itself fails.
//!
//! References:
//!   - MySQL: <https://dev.mysql.com/doc/mysql-errors/8.0/en/server-error-reference.html>
//!   - MariaDB: <https://mariadb.com/kb/en/mariadb-error-codes/>
//!   - mysql crate `consts` module (client-side codes).

#![allow(dead_code)] // Some codes are documented for grep but not yet matched.

// ---------------------------------------------------------------------------
// Server-side errors (ER_*) — returned by the server in mysql::ServerError.
// ---------------------------------------------------------------------------

/// SQL syntax error.
pub const ER_PARSE_ERROR: u16 = 1064;

/// Reference to a non-existent table.
pub const ER_NO_SUCH_TABLE: u16 = 1146;

/// Reference to a non-existent or ambiguous column.
pub const ER_BAD_FIELD_ERROR: u16 = 1054;

/// Authentication failed for the supplied user/password.
pub const ER_ACCESS_DENIED_ERROR: u16 = 1045;

/// Authenticated user has no rights on the requested database.
pub const ER_DBACCESS_DENIED_ERROR: u16 = 1044;

/// Authenticated user has no privileges on the referenced table.
pub const ER_TABLEACCESS_DENIED_ERROR: u16 = 1142;

/// Authenticated user has no privileges on the referenced column.
pub const ER_COLUMNACCESS_DENIED_ERROR: u16 = 1143;

/// Reference to a non-existent database / schema.
pub const ER_BAD_DB_ERROR: u16 = 1049;

// ---------------------------------------------------------------------------
// Client-side errors (CR_*) — surfaced by the mysql client library.
// ---------------------------------------------------------------------------

/// Generic connection error from the client library.
pub const CR_CONNECTION_ERROR: u16 = 2002;

/// Could not reach the server host.
pub const CR_CONN_HOST_ERROR: u16 = 2003;

/// Server has gone away mid-session.
pub const CR_SERVER_GONE_ERROR: u16 = 2006;

/// Lost connection to the server during query execution.
pub const CR_SERVER_LOST: u16 = 2013;
