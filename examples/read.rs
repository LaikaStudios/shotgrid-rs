//! Small example program that prints out the schema for a field on an entity for a given project.
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
//! $ cargo run --example read 'human_user' '60' 'name,created_at,projects'
//! ```

use serde_json::Value;
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity: Option<String> = env::args().skip(1).next().and_then(|s| Some(s));
    let entity_id: Option<String> = env::args().skip(2).next().and_then(|s| Some(s));
    let fields: Option<String> = env::args().skip(3).next().and_then(|s| Some(s));

    println!(
        "Attempting to read {:?} {:?} with fields {:?}",
        entity, entity_id, fields
    );

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .read(
            &token,
            &entity.unwrap(),
            entity_id.unwrap().parse::<i32>().unwrap(),
            Some(&fields.unwrap()),
        )
        .await?;
    for key in resp["data"].as_object().expect("response decode").keys() {
        println!("{}: {}", key, resp["data"][key]);
    }
    Ok(())
}
