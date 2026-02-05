//! System Monitor widget displaying CPU, RAM, and disk usage
//!
//! This widget shows real-time system resource usage using the sysinfo crate.

use std::time::{Duration, Instant};

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing::debug;

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// System Monitor widget showing CPU, RAM, and optionally disk usage
pub struct SystemMonitorWidget {
    system: System,
    last_update: Instant,
    update_interval: Duration,

    // Configuration
    show_cpu: bool,
    show_memory: bool,
    show_disk: bool,

    // Cached values
    cpu_usage: f32,
    memory_used: u64,
    memory_total: u64,
    disk_used: u64,
    disk_total: u64,
}

impl SystemMonitorWidget {
    /// Create a new System Monitor widget
    pub fn new(show_cpu: bool, show_memory: bool, show_disk: bool, update_interval: u64) -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        // Initial refresh
        system.refresh_all();

        // Get initial values
        let cpu_usage = system.global_cpu_info().cpu_usage();
        let memory_used = system.used_memory();
        let memory_total = system.total_memory();

        // Disk info
        let (disk_used, disk_total) = if show_disk {
            Self::get_disk_info()
        } else {
            (0, 0)
        };

        Self {
            system,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(update_interval),
            show_cpu,
            show_memory,
            show_disk,
            cpu_usage,
            memory_used,
            memory_total,
            disk_used,
            disk_total,
        }
    }

    /// Get disk usage information for the root filesystem
    fn get_disk_info() -> (u64, u64) {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();

        // Find the root filesystem
        for disk in disks.list() {
            if disk.mount_point().to_str() == Some("/") {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                return (used, total);
            }
        }

        // Fallback: use the first disk
        if let Some(disk) = disks.list().first() {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            return (used, total);
        }

        (0, 0)
    }

    /// Format bytes as human-readable string
    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1}G", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.0}M", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.0}K", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }

    /// Generate display string
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();

        if self.show_cpu {
            parts.push(format!("CPU: {:.0}%", self.cpu_usage));
        }

        if self.show_memory {
            let mem_percent = if self.memory_total > 0 {
                (self.memory_used as f64 / self.memory_total as f64) * 100.0
            } else {
                0.0
            };
            parts.push(format!(
                "RAM: {}/{} ({:.0}%)",
                Self::format_bytes(self.memory_used),
                Self::format_bytes(self.memory_total),
                mem_percent
            ));
        }

        if self.show_disk && self.disk_total > 0 {
            let disk_percent = (self.disk_used as f64 / self.disk_total as f64) * 100.0;
            parts.push(format!(
                "Disk: {}/{} ({:.0}%)",
                Self::format_bytes(self.disk_used),
                Self::format_bytes(self.disk_total),
                disk_percent
            ));
        }

        if parts.is_empty() {
            "System Monitor".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

impl Widget for SystemMonitorWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "system_monitor",
            name: "System Monitor",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() < self.update_interval {
            return;
        }

        // Refresh system information
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        self.cpu_usage = self.system.global_cpu_info().cpu_usage();
        self.memory_used = self.system.used_memory();
        self.memory_total = self.system.total_memory();

        if self.show_disk {
            let (used, total) = Self::get_disk_info();
            self.disk_used = used;
            self.disk_total = total;
        }

        self.last_update = Instant::now();

        debug!(
            cpu = %self.cpu_usage,
            mem_used = %Self::format_bytes(self.memory_used),
            mem_total = %Self::format_bytes(self.memory_total),
            "System monitor updated"
        );
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }
}

impl Default for SystemMonitorWidget {
    fn default() -> Self {
        Self::new(true, true, false, 2)
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for SystemMonitorWidget
pub struct SystemMonitorWidgetFactory;

impl DynWidgetFactory for SystemMonitorWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "system_monitor"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let show_cpu = config
            .get("show_cpu")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_memory = config
            .get("show_memory")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_disk = config
            .get("show_disk")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(2) as u64;

        debug!(
            show_cpu = %show_cpu,
            show_memory = %show_memory,
            show_disk = %show_disk,
            update_interval = %update_interval,
            "Creating SystemMonitorWidget"
        );

        Ok(Box::new(SystemMonitorWidget::new(
            show_cpu,
            show_memory,
            show_disk,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("show_cpu".to_string(), toml::Value::Boolean(true));
        config.insert("show_memory".to_string(), toml::Value::Boolean(true));
        config.insert("show_disk".to_string(), toml::Value::Boolean(false));
        config.insert("update_interval".to_string(), toml::Value::Integer(2));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(interval) = config.get("update_interval") {
            let interval_val = interval
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("'update_interval' must be an integer"))?;

            if interval_val < 1 {
                anyhow::bail!("'update_interval' must be at least 1 second");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(SystemMonitorWidget::format_bytes(500), "500B");
        assert_eq!(SystemMonitorWidget::format_bytes(1024), "1K");
        assert_eq!(SystemMonitorWidget::format_bytes(1024 * 1024), "1M");
        assert_eq!(
            SystemMonitorWidget::format_bytes(1024 * 1024 * 1024),
            "1.0G"
        );
        assert_eq!(
            SystemMonitorWidget::format_bytes(2 * 1024 * 1024 * 1024),
            "2.0G"
        );
    }

    #[test]
    fn test_system_monitor_creation() {
        let widget = SystemMonitorWidget::default();
        assert_eq!(widget.info().id, "system_monitor");
    }

    #[test]
    fn test_display_string() {
        let widget = SystemMonitorWidget::new(true, true, false, 2);
        let display = widget.display_string();
        assert!(display.contains("CPU:"));
        assert!(display.contains("RAM:"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = SystemMonitorWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "system_monitor");
    }
}
