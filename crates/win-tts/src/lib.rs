#[cfg(target_os = "windows")]
mod tts;

#[cfg(target_os = "windows")]
pub use tts::tts;
