/// NATS subject constants for the tools service.
///
/// Subject naming convention:
///   tools.<tool_group>.jobs.{job_id}      — Job submission queue
///   tools.<tool_group>.progress.{job_id}  — Progress update fan-out
///   tools.scheduler.cleanup               — Cron-triggered cleanup

// ── Job Subjects ──

pub const SCAN_JOBS: &str = "tools.scan.jobs";
pub const SCAN_PROGRESS: &str = "tools.scan.progress";
pub const IMAGE_JOBS: &str = "tools.image.jobs";
pub const IMAGE_PROGRESS: &str = "tools.image.progress";
pub const PDF_JOBS: &str = "tools.pdf.jobs";
pub const PDF_PROGRESS: &str = "tools.pdf.progress";
pub const VIDEO_JOBS: &str = "tools.video.jobs";
pub const VIDEO_PROGRESS: &str = "tools.video.progress";
pub const AUDIO_JOBS: &str = "tools.audio.jobs";
pub const AUDIO_PROGRESS: &str = "tools.audio.progress";

// ── Scheduler Subjects ──

pub const SCHEDULER_CLEANUP: &str = "tools.scheduler.cleanup";

// ── Stream Names ──

pub const STREAM_JOBS: &str = "tools-jobs";
pub const STREAM_PROGRESS: &str = "tools-progress";

// ── Stream Configuration ──

/// Returns the stream configuration for jobs.
/// Max age: 24h, storage: file (persistent on disk).
pub fn jobs_stream_config() -> async_nats::jetstream::stream::Config {
    use async_nats::jetstream::stream::Config;
    Config {
        name: STREAM_JOBS.to_string(),
        subjects: vec![
            "tools.scan.jobs.*".to_string(),
            "tools.image.jobs.*".to_string(),
            "tools.pdf.jobs.*".to_string(),
            "tools.video.jobs.*".to_string(),
            "tools.audio.jobs.*".to_string(),
            "tools.scheduler.>".to_string(),
        ],
        max_age: std::time::Duration::from_secs(24 * 3600),
        storage: async_nats::jetstream::stream::StorageType::File,
        ..Default::default()
    }
}

/// Returns the stream configuration for progress events.
/// Max age: 1h, storage: memory (no persistence needed).
pub fn progress_stream_config() -> async_nats::jetstream::stream::Config {
    use async_nats::jetstream::stream::Config;
    Config {
        name: STREAM_PROGRESS.to_string(),
        subjects: vec![
            "tools.scan.progress.*".to_string(),
            "tools.image.progress.*".to_string(),
            "tools.pdf.progress.*".to_string(),
            "tools.video.progress.*".to_string(),
            "tools.audio.progress.*".to_string(),
        ],
        max_age: std::time::Duration::from_secs(3600),
        storage: async_nats::jetstream::stream::StorageType::Memory,
        ..Default::default()
    }
}

/// Build a job subject for a given tool and job ID.
pub fn job_subject(tool_group: &str, job_id: &str) -> String {
    format!("tools.{}.jobs.{}", tool_group, job_id)
}

/// Build a progress subject for a given tool and job ID.
pub fn progress_subject(tool_group: &str, job_id: &str) -> String {
    format!("tools.{}.progress.{}", tool_group, job_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subject_format() {
        assert_eq!(job_subject("scan", "abc-123"), "tools.scan.jobs.abc-123");
        assert_eq!(
            progress_subject("scan", "abc-123"),
            "tools.scan.progress.abc-123"
        );
        assert_eq!(
            job_subject("image", "def-456"),
            "tools.image.jobs.def-456"
        );
        assert_eq!(SCHEDULER_CLEANUP, "tools.scheduler.cleanup");
    }

    #[test]
    fn test_stream_names() {
        assert_eq!(STREAM_JOBS, "tools-jobs");
        assert_eq!(STREAM_PROGRESS, "tools-progress");
    }
}