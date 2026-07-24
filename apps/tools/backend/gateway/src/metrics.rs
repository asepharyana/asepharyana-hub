use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Simple Prometheus metrics collector.
pub struct Metrics {
    /// Counter: tools_jobs_total{tool, status}
    jobs_total: Mutex<HashMap<(String, String), AtomicU64>>,
    /// Counter: tools_uploaded_files_total{tool, status}
    uploaded_files_total: Mutex<HashMap<(String, String), AtomicU64>>,
    /// Histogram buckets for processing duration (ms)
    duration_buckets: Vec<f64>,
    /// Histogram: tools_processing_duration_ms{tool}
    duration_histogram: Mutex<HashMap<String, Vec<AtomicU64>>>,
    /// Gauge: tools_queue_depth{tool}
    queue_depth: Mutex<HashMap<String, AtomicU64>>,
    /// Counter: tools_rate_limit_hits{tool}
    rate_limit_hits: Mutex<HashMap<String, AtomicU64>>,
    /// Counter: cleanup deleted files
    cleanup_deleted_files: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            jobs_total: Mutex::new(HashMap::new()),
            uploaded_files_total: Mutex::new(HashMap::new()),
            duration_buckets: vec![
                100.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0, 32000.0,
            ],
            duration_histogram: Mutex::new(HashMap::new()),
            queue_depth: Mutex::new(HashMap::new()),
            rate_limit_hits: Mutex::new(HashMap::new()),
            cleanup_deleted_files: AtomicU64::new(0),
        }
    }

    pub fn increment_jobs_total(&self, tool: &str, status: &str) {
        if let Ok(mut map) = self.jobs_total.lock() {
            map.entry((tool.to_string(), status.to_string()))
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn increment_uploaded_files(&self, tool: &str, status: &str) {
        if let Ok(mut map) = self.uploaded_files_total.lock() {
            map.entry((tool.to_string(), status.to_string()))
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    #[allow(unused)]
    pub fn record_duration(&self, tool: &str, duration_ms: f64) {
        if let Ok(mut map) = self.duration_histogram.lock() {
            let entry = map
                .entry(tool.to_string())
                .or_insert_with(|| {
                    (0..self.duration_buckets.len())
                        .map(|_| AtomicU64::new(0))
                        .collect()
                });
            for (i, bucket) in self.duration_buckets.iter().enumerate() {
                if duration_ms <= *bucket {
                    if let Some(b) = entry.get(i) {
                        b.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    }

    pub fn set_queue_depth(&self, tool: &str, depth: u64) {
        if let Ok(mut map) = self.queue_depth.lock() {
            map.entry(tool.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .store(depth, Ordering::Relaxed);
        }
    }

    #[allow(unused)]
    pub fn increment_rate_limit_hits(&self, tool: &str) {
        if let Ok(mut map) = self.rate_limit_hits.lock() {
            map.entry(tool.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    #[allow(unused)]
    pub fn increment_cleanup_deleted(&self) {
        self.cleanup_deleted_files.fetch_add(1, Ordering::Relaxed);
    }

    /// Format all metrics as Prometheus text format.
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str("# HELP tools_jobs_total Total jobs processed\n");
        output.push_str("# TYPE tools_jobs_total counter\n");
        if let Ok(map) = self.jobs_total.lock() {
            for ((tool, status), count) in map.iter() {
                let val = count.load(Ordering::Relaxed);
                output.push_str(&format!(
                    "tools_jobs_total{{tool=\"{}\",status=\"{}\"}} {}\n",
                    tool, status, val
                ));
            }
        }

        output.push_str("# HELP tools_uploaded_files_total Total uploaded files\n");
        output.push_str("# TYPE tools_uploaded_files_total counter\n");
        if let Ok(map) = self.uploaded_files_total.lock() {
            for ((tool, status), count) in map.iter() {
                let val = count.load(Ordering::Relaxed);
                output.push_str(&format!(
                    "tools_uploaded_files_total{{tool=\"{}\",status=\"{}\"}} {}\n",
                    tool, status, val
                ));
            }
        }

        output.push_str("# HELP tools_processing_duration_ms Processing duration histogram\n");
        output.push_str("# TYPE tools_processing_duration_ms histogram\n");
        if let Ok(map) = self.duration_histogram.lock() {
            for (tool, buckets) in map.iter() {
                for (i, bucket) in self.duration_buckets.iter().enumerate() {
                    if let Some(b) = buckets.get(i) {
                        let val = b.load(Ordering::Relaxed);
                        if val > 0 {
                            output.push_str(&format!(
                                "tools_processing_duration_ms_bucket{{tool=\"{}\",le=\"{}\"}} {}\n",
                                tool, bucket, val
                            ));
                        }
                    }
                }
            }
        }

        output.push_str("# HELP tools_queue_depth Current queue depth\n");
        output.push_str("# TYPE tools_queue_depth gauge\n");
        if let Ok(map) = self.queue_depth.lock() {
            for (tool, depth) in map.iter() {
                let val = depth.load(Ordering::Relaxed);
                output.push_str(&format!(
                    "tools_queue_depth{{tool=\"{}\"}} {}\n",
                    tool, val
                ));
            }
        }

        output.push_str("# HELP tools_rate_limit_hits Total rate limit violations\n");
        output.push_str("# TYPE tools_rate_limit_hits counter\n");
        if let Ok(map) = self.rate_limit_hits.lock() {
            for (tool, count) in map.iter() {
                let val = count.load(Ordering::Relaxed);
                output.push_str(&format!(
                    "tools_rate_limit_hits{{tool=\"{}\"}} {}\n",
                    tool, val
                ));
            }
        }

        output.push_str("# HELP tools_cleanup_deleted_files Total files deleted by cleanup\n");
        output.push_str("# TYPE tools_cleanup_deleted_files counter\n");
        output.push_str(&format!(
            "tools_cleanup_deleted_files {}\n",
            self.cleanup_deleted_files.load(Ordering::Relaxed)
        ));

        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}