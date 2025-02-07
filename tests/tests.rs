use std::fmt::Display;

use rusqlite::{ffi, ErrorCode};
use tokio_rusqlite_new::{Connection, Error, Result};

#[tokio::test]
async fn open_in_memory_test() -> Result<()> {
    let conn = Connection::open_in_memory().await;
    assert!(conn.is_ok());
    Ok(())
}

#[tokio::test]
async fn call_success_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let result = conn
        .call(|conn| {
            conn.execute(
                "CREATE TABLE person(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
                [],
            )
        })
        .await?;

    assert_eq!(0, result);

    Ok(())
}

#[tokio::test]
async fn call_unwrap_success_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let result = conn
        .call_unwrap(|conn| {
            conn.execute(
                "CREATE TABLE person(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
                [],
            )
            .unwrap()
        })
        .await;

    assert_eq!(0, result);

    Ok(())
}

#[tokio::test]
async fn call_failure_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let result = conn.call(|conn| conn.execute("Invalid sql", [])).await;

    assert!(match result.unwrap_err() {
        Error::Error(e) => {
            e == rusqlite::Error::SqlInputError {
                error: ffi::Error {
                    code: ErrorCode::Unknown,
                    extended_code: 1,
                },
                msg: "near \"Invalid\": syntax error".to_string(),
                sql: "Invalid sql".to_string(),
                offset: 0,
            }
        }
        _ => false,
    });

    Ok(())
}

#[tokio::test]
async fn close_success_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    assert!(conn.close().await.is_ok());

    Ok(())
}

#[tokio::test]
async fn double_close_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let conn2 = conn.clone();

    assert!(conn.close().await.is_ok());
    assert!(conn2.close().await.is_ok());

    Ok(())
}

#[tokio::test]
async fn close_call_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let conn2 = conn.clone();

    assert!(conn.close().await.is_ok());

    let result = conn2.call(|conn| conn.execute("SELECT 1;", [])).await;

    assert!(matches!(result.unwrap_err(), Error::ConnectionClosed));

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn close_call_unwrap_test() {
    let conn = Connection::open_in_memory().await.unwrap();

    let conn2 = conn.clone();

    assert!(conn.close().await.is_ok());

    conn2
        .call_unwrap(|conn| conn.execute("SELECT 1;", []))
        .await
        .unwrap();
}

#[tokio::test]
async fn close_failure_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    conn.call(|conn| {
        conn.execute(
            "CREATE TABLE person(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
            [],
        )
    })
    .await?;

    conn.call(|conn| {
        // Leak a prepared statement to make the database unclose-able
        // See https://www.sqlite.org/c3ref/close.html for details regarding this behaviour
        let stmt = Box::new(conn.prepare("INSERT INTO person VALUES (1, ?1);").unwrap());
        Box::leak(stmt);
        Ok(())
    })
    .await?;

    assert!(match conn.close().await.unwrap_err() {
        Error::Close((_, e)) => {
            e == rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ErrorCode::DatabaseBusy,
                    extended_code: 5,
                },
                Some(
                    "unable to close due to unfinalized statements or unfinished backups"
                        .to_string(),
                ),
            )
        }
        _ => false,
    });

    Ok(())
}

#[tokio::test]
async fn debug_format_test() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    assert_eq!("Connection".to_string(), format!("{conn:?}"));

    Ok(())
}

#[tokio::test]
async fn test_error_display() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let error: Error = Error::Close((conn, rusqlite::Error::InvalidQuery));
    assert_eq!(
        "Close((Connection, \"Query is not read-only\"))",
        format!("{error}")
    );

    let error: Error = Error::ConnectionClosed;
    assert_eq!("ConnectionClosed", format!("{error}"));

    let error = Error::Error(rusqlite::Error::InvalidQuery);
    assert_eq!("Error(\"Query is not read-only\")", format!("{error}"));

    Ok(())
}

#[tokio::test]
async fn test_error_source() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let error: Error = Error::Close((conn, rusqlite::Error::InvalidQuery));
    assert_eq!(
        std::error::Error::source(&error)
            .and_then(|e| e.downcast_ref::<rusqlite::Error>())
            .unwrap(),
        &rusqlite::Error::InvalidQuery,
    );

    let error: Error = Error::ConnectionClosed;
    assert_eq!(
        std::error::Error::source(&error).and_then(|e| e.downcast_ref::<rusqlite::Error>()),
        None,
    );

    let error = Error::Error(rusqlite::Error::InvalidQuery);
    assert_eq!(
        std::error::Error::source(&error)
            .and_then(|e| e.downcast_ref::<rusqlite::Error>())
            .unwrap(),
        &rusqlite::Error::InvalidQuery,
    );

    Ok(())
}

fn failable_func(_: &rusqlite::Connection) -> std::result::Result<(), MyError> {
    Err(MyError::MySpecificError)
}

#[tokio::test]
async fn test_ergonomic_errors() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let err = conn.call(|conn| failable_func(conn)).await.unwrap_err();

    assert!(matches!(err, Error::Error(MyError::MySpecificError)));

    conn.call(|_conn| Ok(())).await?;

    Ok(())
}

#[tokio::test]
async fn test_call_raw() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let err = conn.call_raw(|_conn| Err(())).await.expect("call success");

    assert!(matches!(err, Err::<(), ()>(())));

    let ok = conn.call_raw(|_conn| Ok(())).await.expect("call success");

    assert!(matches!(ok, Ok::<(), ()>(())));

    let conn2 = conn.clone();
    conn.close().await?;

    let err: Result<Result<()>> = conn2.call_raw(|_conn| Ok(())).await;

    assert!(matches!(err, Err(crate::Error::ConnectionClosed)));

    Ok(())
}

// The rest is boilerplate, not really that important

#[derive(Debug)]
enum MyError {
    MySpecificError,
}

impl Display for MyError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl std::error::Error for MyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
