//! Small example program that prints out a table of projects. For this to work you must set 3 env
//! vars, `SG_SERVER`, `SG_SCRIPT_NAME`, and `SG_SCRIPT_KEY`.
//!
//! Set the `SG_SERVER` environment variable to the url for your shotgun server, eg:
//!
//! ```text
//! export SG_SERVER=https://shotgun.example.com
//! ```
//!
//! `shotgun_rs` also looks at the `CA_BUNDLE` environment variable for when you need a custom CA
//! loaded to access your shotgun server, for example:
//!
//! ```text
//! export CA_BUNDLE=/etc/ssl/my-ca-certs.crt
//! ```
//!
//! Usage:
//!
//! ```text
//! $ cargo run --example list-projects
//! ```

#[macro_use]
extern crate prettytable;
use prettytable::{format, Table};
use serde_json::{json, Value};
use shotgun_rs::{Entity, Filters, Shotgun};
use std::env;
use tokio::prelude::*;

fn main() {
    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let fut = {
        let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

        sg.authenticate_script()
            .and_then(|mut resp: Value| {
                let val = resp["access_token"].take();
                Ok(val.as_str().unwrap().to_string())
            })
            .and_then(move |token: String| {
                sg.search(
                    &token,
                    Entity::Project,
                    "id,code,name",
                    Filters::Array(&json!({ "filters": [] })),
                    None,
                    None,
                    None,
                )
                .and_then(|resp: Value| Ok(resp["data"].as_array().unwrap().to_vec()))
                .and_then(|items| {
                    let mut table = Table::new();
                    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                    table.set_titles(row!["ID", "Code", "Name"]);

                    for project in items {
                        let id = project["id"].as_i64().unwrap();
                        let code = project["attributes"]["code"].as_str().unwrap_or("");
                        let name = project["attributes"]["name"].as_str().unwrap_or("");
                        table.add_row(row![id, code, name]);
                    }

                    table.printstd();
                    Ok(())
                })
            })
            .map_err(|e| {
                eprintln!("\nSomething bad happend:\n{}", e);
            })
    };

    // Execute the future pipeline, blocking until it completes.
    tokio::run(fut);
}
