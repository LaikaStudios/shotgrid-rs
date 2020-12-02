//! Small example program that prints out the relationship for an entity.
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
//! ```text
//! $ cargo run --example entity-relationship-read asset 1234 sg_assets_1
//! ```
//!
//! This example does not take any arguments for the optional parameters.
//!

use serde_json::Value;
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();
    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity: Option<String> = env::args().nth(1);
    let entity_id: Option<i32> = env::args().nth(2).map(|s| s.parse().expect("Entity ID"));
    let related_field: Option<String> = env::args().nth(3);

    println!(
        "Attempting to read {:?} {:?} for field {:?}",
        entity, entity_id, related_field
    );

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .entity_relationship_read(
            &token,
            &entity.unwrap(),
            entity_id.unwrap(),
            &related_field.unwrap(),
        )
        .execute()
        .await?;

    for entry in resp["data"].as_array().expect("response decode data") {
        println!("{}", entry)
    }

    for key in resp["links"]
        .as_object()
        .expect("response decode links")
        .keys()
    {
        println!("Link {}: {}", key, resp["links"][key])
    }

    Ok(())
}
