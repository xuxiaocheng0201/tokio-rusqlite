# Tokio-Rusqlite-New

[![Crate](https://img.shields.io/crates/v/tokio-rusqlite-new.svg)](https://crates.io/crates/tokio-rusqlite-new)
[![GitHub last commit](https://img.shields.io/github/last-commit/xuxiaocheng0201/tokio-rusqlite)](https://github.com/xuxiaocheng0201/tokio-rusqlite/commits/master)
[![GitHub issues](https://img.shields.io/github/issues-raw/xuxiaocheng0201/tokio-rusqlite)](https://github.com/xuxiaocheng0201/tokio-rusqlite/issues)
[![GitHub pull requests](https://img.shields.io/github/issues-pr/xuxiaocheng0201/tokio-rusqlite)](https://github.com/xuxiaocheng0201/tokio-rusqlite/pulls)
[![GitHub](https://img.shields.io/github/license/xuxiaocheng0201/tokio-rusqlite)](https://github.com/xuxiaocheng0201/tokio-rusqlite/blob/master/LICENSE)

## Description

Asynchronous handle for rusqlite library.

This is a fork of [tokio-rusqlite](https://github.com/programatik29/tokio-rusqlite).

## Guide

This library provides [`Connection`] struct.
The [`Connection`] struct is a handle to call functions in background thread and can be cloned cheaply.
[`Connection::call`] method calls provided function in the background thread and returns its result asynchronously.

[`Connection`]: https://docs.rs/tokio-rusqlite-new/latest/tokio_rusqlite_new/struct.Connection.html
[`Connection::call`]: https://docs.rs/tokio-rusqlite-new/latest/tokio_rusqlite_new/struct.Connection.html#method.call

## Design

A thread is spawned for each opened connection handle. When `call` method
is called: provided function is boxed, sent to the thread through mpsc
channel and executed. Return value is then sent by oneshot channel from
the thread and then returned from function.

## Usage

```rust
use tokio_rusqlite_new::{params, Connection, Result};

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let conn = Connection::open_in_memory().await?;

    let people = conn.call(|conn| {
        conn.execute(
            "CREATE TABLE person (
                id    INTEGER PRIMARY KEY,
                name  TEXT NOT NULL,
                data  BLOB
            )",
            [],
        )?;

        let steven = Person {
            id: 1,
            name: "Steven".to_string(),
            data: None,
        };

        conn.execute(
            "INSERT INTO person (id, name, data) VALUES (?1, ?2, ?3)",
            params![steven.id, steven.name, steven.data],
        )?;

        let mut stmt = conn.prepare("SELECT id, name, data FROM person")?;
        let people = stmt
            .query_map([], |row| {
                Ok(Person {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    data: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<Person>, rusqlite::Error>>()?;

        Ok(people)
    })
    .await?;

    for person in people {
        println!("Found person {person:?}");
    }

    conn.close().await?;
    Ok(())
}
```

## Safety

This crate uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.

## License

This project is licensed under the [MIT license](./LICENSE).
