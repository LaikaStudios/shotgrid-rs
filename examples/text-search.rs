//! Small example program that prints out the list of entity types for a given project.
//!
//! For this to work you must set 3 env
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
//! $ cargo run --example text-search <ShotGrid login to search as> <asset name to search for> [limit]
//! ```

use serde_json::Value;
use shotgrid_rs::filters::{self, field};
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let login: String = env::args().nth(1).expect("login required");
    let text: String = env::args().nth(2).expect("search text required");
    let limit: Option<usize> = env::args()
        .nth(3)
        .map(|s| s.parse().expect("limit must be a number"));

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let sess = sg.authenticate_script_as_user(&login).await?;

    let entity_filters = vec![(
        "Asset",
        filters::basic(&[field("sg_status_list").is_not("omt")]),
    )]
    .into_iter()
    .collect();

    let resp: Value = sess
        .text_search(Some(&text), entity_filters)
        .size(limit)
        .execute()
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    Ok(())
}
