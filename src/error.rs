use std::fmt::Display;
use crate::Connection;

pub(crate) const BUG_TEXT: &str = "bug in tokio-rusqlite-new, please report";

#[derive(Debug)]
/// Represents the errors specific for this library.
#[non_exhaustive]
pub enum Error<E = rusqlite::Error> {
    /// The connection to the SQLite has been closed and cannot be queried anymore.
    ConnectionClosed,

    /// An error occurred while closing the SQLite connection.
    /// This `Error` variant contains the [`Connection`], which can be used to retry the close operation
    /// and the underlying [`rusqlite::Error`] that made it impossible to close the database.
    Close((Connection, rusqlite::Error)),

    /// An application-specific error occurred.
    Error(E),
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConnectionClosed => write!(f, "ConnectionClosed"),
            Error::Close((_, e)) => write!(f, "Close((Connection, \"{e}\"))"),
            Error::Error(e) => write!(f, "Error(\"{e}\")"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ConnectionClosed => None,
            Error::Close((_, e)) => Some(e),
            Error::Error(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Error(value)
    }
}

/// The result returned on method calls in this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
