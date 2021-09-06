//! Small example program that updates a field on an entity.
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
//! $ cargo run --example schema-field-update task sg_hello name hello_world
//! ```

use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity_type = env::args().nth(1).expect("entity type");
    let field_name = env::args().nth(2).expect("field name");
    let property_name = env::args().nth(3).expect("property name");
    let value = env::args().nth(4).expect("property value");

    println!("Attempting to update {} on {}", field_name, entity_type);

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let sess = sg.authenticate_script().await?;
    let resp = sess
        .schema_field_update(
            &entity_type,
            &field_name,
            vec![(property_name, value)],
            None,
        )
        .await?;

    println!("Data: {:?}", resp.data);
    println!("Links: {:?}", resp.links);

    Ok(())
}
