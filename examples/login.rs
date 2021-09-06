//! Small example program that runs a login using a username and password.
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
//! $ cargo run --example login -- <username>
//! ```

extern crate shotgrid_rs;
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    // Get a username from argv.
    let username = env::args()
        .nth(1)
        .expect("Please specify a user to login as");

    // Prompt the user for a password.
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let sg = Client::new(
        env::var("SG_SERVER").expect("SG_SERVER is required."),
        None,
        None,
    )
    .expect("SG Client");

    let _sess = sg.authenticate_user(&username, &password).await?;
    println!("Login Succeeded!");
    Ok(())
}
