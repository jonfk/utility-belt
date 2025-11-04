use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressManager;

impl ProgressManager {
    pub fn setup() -> MultiProgress {
        MultiProgress::new()
    }

    pub fn create_scan_progress(multi: &MultiProgress, message: &str) -> ProgressBar {
        let scan_pb = multi.add(ProgressBar::new_spinner());
        scan_pb.set_style(
            ProgressStyle::with_template("🔍 {spinner:.green} {wide_msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        scan_pb.set_message(message.to_string());
        scan_pb
    }

    pub fn create_process_progress(multi: &MultiProgress, total: u64) -> ProgressBar {
        let process_pb = multi.add(ProgressBar::new(total));
        process_pb.set_style(
            ProgressStyle::with_template(
                "📁 [{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        process_pb
    }

    pub fn create_cleanup_progress(multi: &MultiProgress, total: u64) -> ProgressBar {
        let process_pb = multi.add(ProgressBar::new(total));
        process_pb.set_style(
            ProgressStyle::with_template(
                "🗑️  [{elapsed}] {bar:40.red/yellow} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        process_pb
    }

    pub fn create_copy_progress(multi: &MultiProgress, total: u64) -> ProgressBar {
        let process_pb = multi.add(ProgressBar::new(total));
        process_pb.set_style(
            ProgressStyle::with_template(
                "📋 [{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        process_pb
    }
}
