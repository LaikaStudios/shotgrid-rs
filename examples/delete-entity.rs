//! Small example program that deletes an entity. Be careful.
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
//! $ cargo run --example delete-entity task 123456
//! ```

use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity: Option<String> = env::args().nth(1);
    let entity_id: Option<i32> = env::args()
        .nth(2)
        .and_then(|s| Some(s.parse().expect("Entity ID")));

    println!("Attempting to delete {:?} {:?}", entity, entity_id);

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let session = sg.authenticate_script().await?;

    let resp = session
        .destroy(&entity.unwrap(), entity_id.unwrap())
        .await?;
    println!(
        "If there's not an error here, then you should be good: {:?}",
        resp
    );
    Ok(())
}
