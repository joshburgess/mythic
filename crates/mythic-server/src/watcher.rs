//! File watcher with debounced events for triggering rebuilds.

use anyhow::{Context, Result};
use mythic_core::config::SiteConfig;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Types of file changes detected by the watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A content file (.md) changed — incremental rebuild.
    ContentChanged(PathBuf),
    /// A template file changed — full rebuild needed.
    TemplateChanged(PathBuf),
    /// A CSS file changed — hot CSS reload possible.
    CssChanged(PathBuf),
    /// The config file changed — full reload + rebuild.
    ConfigChanged,
}

/// Watches content, template, and data directories for changes.
pub struct FileWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    pub rx: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Start watching directories specified in the config.
    pub fn new(config: &SiteConfig, root: &Path) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let content_dir = root.join(&config.content_dir);
        let template_dir = root.join(&config.template_dir);
        let data_dir = root.join(&config.data_dir);
        let config_file = root.join("mythic.toml");

        let event_tx = tx.clone();
        let content_dir_c = content_dir.clone();
        let template_dir_c = template_dir.clone();
        let config_file_c = config_file.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(200),
            move |result: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                let events = match result {
                    Ok(evs) => evs,
                    Err(_) => return,
                };

                for event in events {
                    if event.kind != DebouncedEventKind::Any {
                        continue;
                    }

                    let path = &event.path;

                    let watch_event = if path == &config_file_c {
                        WatchEvent::ConfigChanged
                    } else if path.starts_with(&content_dir_c) {
                        match path.extension().and_then(|e| e.to_str()) {
                            Some("css") => WatchEvent::CssChanged(path.clone()),
                            _ => WatchEvent::ContentChanged(path.clone()),
                        }
                    } else if path.starts_with(&template_dir_c) {
                        match path.extension().and_then(|e| e.to_str()) {
                            Some("css") => WatchEvent::CssChanged(path.clone()),
                            _ => WatchEvent::TemplateChanged(path.clone()),
                        }
                    } else {
                        WatchEvent::ContentChanged(path.clone())
                    };

                    let _ = event_tx.send(watch_event);
                }
            },
        )
        .context("Failed to create file watcher")?;

        // Watch directories that exist
        for dir in [&content_dir, &template_dir, &data_dir] {
            if dir.exists() {
                debouncer
                    .watcher()
                    .watch(dir, notify::RecursiveMode::Recursive)
                    .with_context(|| format!("Failed to watch: {}", dir.display()))?;
            }
        }

        // Watch config file
        if config_file.exists() {
            debouncer
                .watcher()
                .watch(&config_file, notify::RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch: {}", config_file.display()))?;
        }

        Ok(FileWatcher {
            _debouncer: debouncer,
            rx,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mythic_core::config::SiteConfig;
    use std::path::PathBuf;

    fn test_config() -> SiteConfig {
        SiteConfig::for_testing("Test", "http://localhost")
    }

    #[test]
    fn detects_content_change() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("test.md"), "# Hello").unwrap();

        let config = test_config();
        let watcher = FileWatcher::new(&config, dir.path()).unwrap();

        // Modify the file
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(content.join("test.md"), "# Updated").unwrap();

        // Drain events until we find the expected ContentChanged for test.md.
        // On some platforms (macOS), the watcher may fire for initial setup
        // events before delivering the modification event.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                panic!("Timed out waiting for ContentChanged(test.md)");
            }
            match watcher.rx.recv_timeout(remaining) {
                Ok(WatchEvent::ContentChanged(p)) if p.ends_with("test.md") => break,
                Ok(_) => continue, // skip unrelated events
                Err(_) => panic!("Timed out waiting for ContentChanged(test.md)"),
            }
        }
    }

    #[test]
    fn debouncing_collapses_rapid_changes() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("test.md"), "v1").unwrap();

        let config = test_config();
        let watcher = FileWatcher::new(&config, dir.path()).unwrap();

        // Rapid-fire writes
        std::thread::sleep(Duration::from_millis(50));
        for i in 0..5 {
            std::fs::write(content.join("test.md"), format!("v{}", i + 2)).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }

        // Should get a small number of events (debounced), not 5
        std::thread::sleep(Duration::from_millis(500));
        let mut count = 0;
        while watcher.rx.try_recv().is_ok() {
            count += 1;
        }
        // Debouncing should collapse rapid writes; we expect fewer events than writes
        assert!(count <= 3, "Expected debounced events, got {count}");
    }
}
