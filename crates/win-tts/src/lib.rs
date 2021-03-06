#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub mod bindings {
    ::windows::include_bindings!();
}

#[cfg(target_os = "windows")]
mod tts;

#[cfg(target_os = "windows")]
pub use tts::tts;
