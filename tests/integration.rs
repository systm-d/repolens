//! Integration tests entry point.
//!
//! Each submodule is compiled into this single test binary. Adding new
//! integration test groups means creating a file under `tests/integration/`
//! and declaring it here.

mod integration {
    pub mod completions;
}
