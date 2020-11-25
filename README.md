# shotgun-rs

`shotgun-rs` is a REST API client for [Autodesk Shotgun][shotgun] built with
[reqwest] and [serde_json].

## Usage

The general pattern of usage starts with a `shotgun_rs::Shotgun` client.

```rust
let server = "https://my-shotgun.example.com".to_string();
let script_name = "my-api-user";
let script_key = "********";
let sg = Shotgun::new(server, Some(script_name), Some(script_key))?;
```

Once your client is in hand, you'd use one of the authentication methods to
get an `access_token`.

```rust
// Authenticates using the script name and script key held by the client.
let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
```

From there, you can pass that access token around to the various query methods.
For operations where the schema of the response is flexible (based on the
entity type and return fields specified), we use generics to allow the
caller to unpack the response into the type of their choosing. The type just
needs to implement [serde]'s `Deserialize` trait. A number of structs are
provided (ex: `TokenResponse`) to cover responses that are pretty much the
same for everybody.
Others structs are generic over types deeper in the data structure.
For example, `ResourceArrayResponse<R, L>` is generic over `R`
(the resource) which is the items in the array portion of the response, and
`L` which is the type for the response's "links" key).

```rust
use serde_derive::Deserialize;
use serde_json::json;
use shotgun_rs::types::{PaginationLinks, ResourceArrayResponse, SelfLink};
use shotgun_rs::{Shotgun, TokenResponse};

/// This struct should match the return fields specified for the search.
#[derive(Debug, Clone, Deserialize)]
struct ProjectAttrs {
    code: Option<String>,
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Project {
    id: Option<i32>,
    r#type: Option<String>,
    attributes: Option<ProjectAttrs>,
    links: Option<SelfLink>,
}

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    let server = "https://my-shotgun.example.com";
    let script_name = "my-api-user";
    let script_key = "********";

    let sg = Shotgun::new(server.to_string(), Some(script_name), Some(script_key))?;

    let TokenResponse { access_token, .. } = sg.authenticate_script().await?;

    let return_fields = ["id", "code", "name"].join(",");

    // Using type ascription (or a turbofish), we tell search() how to
    // deserialize the response.
    let resp: ResourceArrayResponse<Project, PaginationLinks> = sg
        .search(&access_token, "Project", &return_fields, &json!([]))?
        .size(Some(3))
        .execute()
        .await?;

    let items = resp.data.unwrap_or_default();
    for project in items {
        println!("{:?}", project);
    }
    Ok(())
}
```

For times where you don't want to bother defining structs to represent the
response, you can always deserialize to a `serde_json::Value` and interrogate
the value yourself.

## Logging

The `shotgun_rs` crate offers some logging, though most of it relates to the
internals of the library itself.

If you're interested in logging the HTTP-transport layer, since we're using
[reqwest], you can get some visibility into the transport layer by setting
`reqwest` to `DEBUG`.

Please refer to the docs for your logger crate to see how to adjust log levels
for crates and modules.

## Running Tests

You can run the basic unit test suite via:

```
$ cargo test
```

In addition to the unit tests, there is a set of end-to-end tests (ie, requires
a live Shotgun server) which can be run by enabling the `integration-tests`
feature:

```
$ cargo test --features integration-tests
```

The integration tests require a set of environment vars to be set in order to pass:

- `TEST_SG_SERVER`, the shotgun server to connect to.
- `TEST_SG_SCRIPT_NAME`, the name of an ApiUser to connect as.
- `TEST_SG_SCRIPT_KEY`, the API key to go with the name.
- `TEST_SG_HUMAN_USER_LOGIN`, certain tests require a `HumanUser` so this is
  the login to "sudo as" for those tests.
- `TEST_SG_PROJECT_ID`, some tests require a project to filter by.

At the time of writing, these tests read but don't write. This may change in the
future so please take care when setting these vars.

If possible you may want to isolate your test runs to a secondary shotgun server
(if you have a spare for development), or at the very least select a "test"
project.

[shotgun]: https://www.shotgunsoftware.com/
[reqwest]: https://crates.io/crates/reqwest
[serde]: https://crates.io/crates/serde
[serde_json]: https://crates.io/crates/serde_json
