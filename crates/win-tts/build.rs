fn main() {
    #[cfg(target_os = "windows")]
    windows::build!(
        Windows::Foundation::*,
        Windows::Storage::Streams::*,
        Windows::Media::SpeechSynthesis::*,
    );
}
