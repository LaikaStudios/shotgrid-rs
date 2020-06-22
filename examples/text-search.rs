//! Small example program that prints out the list of entity types for a given project.
//!
//! For this to work you must set 3 env
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
//! $ cargo run --example text-search <shotgun login to search as> <asset name to search for> [limit]
//! ```

use serde_json::{json, Value};
use shotgun_rs::{PaginationParameter, Shotgun};
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let mut args = std::env::args().skip(1);

    let login: String = args.next().expect("login required");
    let text: String = args.next().expect("search text required");
    let limit: Option<usize> = args
        .next()
        .map(|s| s.parse().expect("limit must be a number"));
    let page_req = PaginationParameter {
        number: Some(1),
        size: limit,
    };

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script_as_user(&login).await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .text_search(
            &token,
            &json!({
                "text": &text,
                "entity_types": {
                    "Asset": [["sg_status_list", "is_not", "omt"]]
                },
            }),
            Some(page_req),
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    Ok(())
}
