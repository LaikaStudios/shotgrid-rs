//! Small example program that prints out a table of projects. For this to work you must set 3 env
//! vars, `SG_SERVER`, `SG_SCRIPT_NAME`, and `SG_SCRIPT_KEY`.
//!
//! Set the `SG_SERVER` environment variable to the url for your ShotGrid server, eg:
//!
//! ```text
//! export SG_SERVER=https://shotgrid.example.com
//! ```
//!
//! `shotgrid_rs` also looks at the `CA_BUNDLE` environment variable for when
//! you need a custom CA loaded to access your ShotGrid server, for example:
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
use serde_json::Value;
use shotgrid_rs::filters;
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let sess = sg.authenticate_script().await?;

    let resp: Value = sess
        .search("Project", "id,code,name", &filters::empty())
        .size(Some(3))
        .number(Some(2))
        .execute()
        .await?;
    let items = resp["data"].as_array().unwrap().to_vec();
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
}
