fn main() {
    #[cfg(target_os = "windows")]
    windows::build!(
        windows::foundation::*,
        windows::storage::streams::*,
        windows::media::speechsynthesis::*,
    );
}
