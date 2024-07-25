//! # A binary crate

// Lints:
#![warn(clippy::pedantic)]
#![warn(deprecated_in_future)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use anyhow::{bail, Context, Error};
use dotenv::dotenv;
use human_panic::setup_panic;
use odbc_api::{buffers::TextRowSet, ConnectionOptions, Cursor, Environment, ResultSetMetadata};
use std::env;
use std::fs;
use std::io::stdout;

const BATCH_SIZE: usize = 5000;

fn main() -> Result<(), Error> {
    setup_panic!();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        bail!(
            "Usage: {} <file|query> <path to SQL script file|SQL query>",
            args[0]
        );
    }
    let mode = &args[1];
    let input = &args[2];

    dotenv().context("Failed to find .env file")?;
    let data_source = env::var("DSN").context("DSN must be set in .env file")?;
    let user = env::var("USER").context("USER must be set in .env file")?;
    let password = env::var("PASSWORD").context("PASSWORD must be set in .env file")?;

    let sql_script = match mode.as_str() {
        "file" => fs::read_to_string(input)
            .with_context(|| format!("Failed to read SQL script from file '{input}'"))?,
        "query" => input.to_string(),
        _ => bail!("Invalid mode: {mode}. Use 'file' or 'query'"),
    };

    let out = stdout();
    let mut writer = csv::Writer::from_writer(out);
    let environment = Environment::new()?;

    eprintln!("Connecting: DSN={} User={}", &data_source, &user);

    let connection =
        environment.connect(&data_source, &user, &password, ConnectionOptions::default())?;

    match connection.execute(&sql_script, ())? {
        Some(mut cursor) => {
            let headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            writer.write_record(headline)?;

            let mut buffers = TextRowSet::for_cursor(BATCH_SIZE, &mut cursor, Some(4096))?;
            let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;

            while let Some(batch) = row_set_cursor.fetch()? {
                for row_index in 0..batch.num_rows() {
                    let record = (0..batch.num_cols())
                        .map(|col_index| batch.at(col_index, row_index).unwrap_or(&[]));
                    writer.write_record(record)?;
                }
            }
        }
        None => {
            eprintln!("Query came back empty. No output has been created.");
        }
    }

    Ok(())
}
