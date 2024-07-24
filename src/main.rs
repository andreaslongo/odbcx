//! # A binary crate

// Lints:
#![warn(clippy::pedantic)]
#![warn(deprecated_in_future)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use anyhow::Error;
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
        eprintln!(
            "Usage: {} <file|query> <path to SQL script file|SQL query>",
            args[0]
        );
        std::process::exit(1);
    }
    let mode = &args[1];
    let input = &args[2];

    dotenv().ok();

    let data_source = env::var("DSN").expect("DSN must be set");

    let user = env::var("USER").expect("USER must be set");

    let password = env::var("PASSWORD").expect("PASSWORD must be set");

    eprintln!("DSN: {} User: {}", &data_source, &user);

    let sql_script = if mode == "file" {
        fs::read_to_string(input).expect("Could not read SQL script file")
    } else if mode == "query" {
        input.to_string()
    } else {
        eprintln!("Invalid mode: {mode}. Use 'file' or 'query'");
        std::process::exit(1);
    };

    let out = stdout();
    let mut writer = csv::Writer::from_writer(out);

    let environment = Environment::new()?;

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
