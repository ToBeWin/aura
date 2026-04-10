use sysinfo::System;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ResourceMonitor {
    system: Arc<Mutex<System>>,
    memory_threshold_mb: u64,
    cpu_threshold_percent: f32,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            memory_threshold_mb: 500, // 500MB threshold
            cpu_threshold_percent: 80.0, // 80% CPU threshold
        }
    }

    pub async fn check_resources(&self) -> ResourceStatus {
        let mut sys = self.system.lock().await;
        sys.refresh_all();

        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let available_memory = total_memory - used_memory;
        let available_memory_mb = available_memory / 1024 / 1024;

        let cpu_usage = sys.global_cpu_info().cpu_usage();

        ResourceStatus {
            available_memory_mb,
            cpu_usage,
            should_downgrade: available_memory_mb < self.memory_threshold_mb 
                || cpu_usage > self.cpu_threshold_percent,
        }
    }

    pub async fn suggest_model(&self) -> String {
        let status = self.check_resources().await;

        if status.available_memory_mb < 300 {
            "qwen3.5:0.8b".to_string() // Smallest model
        } else if status.available_memory_mb < 1000 {
            "qwen3.5:2b".to_string() // Medium model
        } else {
            "qwen3.5:9b".to_string() // Largest model
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceStatus {
    pub available_memory_mb: u64,
    #[allow(dead_code)]
    pub cpu_usage: f32,
    pub should_downgrade: bool,
}
