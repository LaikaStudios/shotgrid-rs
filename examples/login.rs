//! Small example program that runs a login using a username and password and prints out the
//! resulting response from shotgun.
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
//! $ cargo run --example login -- <username>
//! ```

extern crate shotgun_rs;
use shotgun_rs::TokenResponse;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    // Get a username from argv.
    let username = env::args()
        .skip(1)
        .take(1)
        .next()
        .expect("Please specify a user to login as");

    // Prompt the user for a password.
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let sg = shotgun_rs::Shotgun::new(
        env::var("SG_SERVER").expect("SG_SERVER is required."),
        None,
        None,
    )
        .expect("SG Client");

    // The receiver can use a type hint here to tell the client how to deserialize the
    // response from the server.
    //
    // The library provides a struct, `TokenResponse`, which fits the shape of a
    // successful auth attempt, but you can of course provide your own deserialization
    // target if you only need a subset of the fields.
    // For this simple case, we could even use a plain `serde_json::Value`.
    let resp: TokenResponse = sg.authenticate_user(&username, &password).await?;
    println!("\nLogin Succeeded:\n{:?}", resp);
    Ok(())
}
