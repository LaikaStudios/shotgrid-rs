# shotgun-rs

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
