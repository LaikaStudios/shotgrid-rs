# v0.2.0 (Unreleased)


## Client initialization

Previously `Shotgun::new()` and `Shotgun::with_client()` accepted api name/key
parameters as `Option<String>`, but now accept a more relaxed `Option<&str>`.

## Revised error handling.

All methods now return results/futures with `ShotgunError`.

The outliers were largely the methods used to initialize the `Shotgun`
struct which previously returned `Result<Shotgun, failure::Error>`
but now use the `ShotgunError` type instead.

In addition to this, internal APIs which result in the processing of HTTP
Responses from Shotgun previously treated the response bodies optimistically,
assuming the deserialization target would match the caller's specified shape.

This often resulted in an error which is less useful when the response from
Shotgun is feedback explaining why a request failed. The reported error would
simply cite that the payload wasn't of the expected shape. Instead we are now
checking the response for an `errors` key, and when present we will parse the
feedback being given by Shotgun and return that as the error.


# [v0.1.0](https://github.com/LaikaStudios/shotgrid-rs/tree/v0.1.0) (2019-04-26)

Initial release.

- Basic crud operations.
- User login and script authentication.
- Search.
