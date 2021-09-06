//! Small example program that makes a user unfollow an entity.
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
//! $ cargo run --example entity-unfollow-update 1023 task 123456
//! ```

use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let user_id: Option<i32> = env::args().nth(1).map(|s| s.parse().expect("User ID"));
    let entity_type: Option<String> = env::args().nth(2);
    let entity_id: Option<i32> = env::args().nth(3).map(|s| s.parse().expect("Entity ID"));

    println!(
        "Attempting to make user {:?} unfollow {:?} {:?}",
        user_id, entity_type, entity_id
    );

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let session = sg.authenticate_script().await?;

    session
        .entity_unfollow_update(user_id.unwrap(), &entity_type.unwrap(), entity_id.unwrap())
        .await?;

    // Returns 204, nothing to print out
    Ok(())
}
