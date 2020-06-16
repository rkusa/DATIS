// This creates sole purpose is to reduce the compile time by not doing the WinRT import in the
// actual crate that is using the API.

#[cfg(target_os = "windows")]
winrt::import!(
    dependencies
        os
    types
        windows::foundation::*
        windows::storage::streams::*
        windows::media::speechsynthesis::*
        windows::system::*
);

#[cfg(target_os = "windows")]
pub use windows::*;
#[cfg(target_os = "windows")]
pub use winrt::{Error, Result, RuntimeType};
