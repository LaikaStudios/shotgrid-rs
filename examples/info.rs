//! Small example program that prints out the ShotGrid server information.
//!
//! For this to work you must set 1 env - this does not require authentication
//! vars, `SG_SERVER`
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
//! $ cargo run --example info
//! ```

use serde_json::Value;
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();
    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let sg = Client::new(server, None, None).expect("SG Client");
    let resp: Value = sg.info().await?;

    for key in resp["data"].as_object().expect("response decode").keys() {
        println!("{}: {}", key, resp["data"][key]);
    }
    Ok(())
}
