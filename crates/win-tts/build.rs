fn main() {
    #[cfg(target_os = "windows")]
    windows::build!(
        windows::storage::streams::*,
        windows::media::speechsynthesis::*,
    );
}
