//! Small example program that makes a user follow an entity.
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
//! // NOTE: The entity type has to be TitleCased here, it can not be snake_cased.
//! $ cargo run --example entity-follow-update 1023 Task 123456
//! ```

use serde_json::Value;
use shotgun_rs::types::EntityIdentifier;
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let user_id: Option<i32> = env::args().nth(1).map(|s| s.parse().expect("User ID"));
    let entity_type: Option<String> = env::args().nth(2);
    let entity_id: Option<i32> = env::args().nth(3).map(|s| s.parse().expect("Entity ID"));

    println!(
        "Attempting to make user {:?} follow {:?} {:?}",
        user_id, entity_type, entity_id
    );

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let entity_identifier = EntityIdentifier {
        record_id: entity_id,
        entity: entity_type,
    };

    sg.entity_follow_update(&token, user_id.unwrap(), vec![entity_identifier])
        .await?;

    // Returns 204, nothing to print out
    Ok(())
}