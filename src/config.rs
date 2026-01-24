use std::{env, path::PathBuf, process::Command};

use crate::error::{Error, Result};

/// Configuration loaded from .taskrc and environment
#[derive(Debug, Clone)]
pub struct Config {
    pub habitica_user_id: String,
    pub habitica_api_key: String,
    pub task_note_dir: PathBuf,
    pub task_note_prefix: String,
    pub task_note_extension: String,
    pub data_location: PathBuf,
    pub verbose: bool,
}

impl Config {
    /// Load configuration from Taskwarrior's config
    pub fn load(verbose: bool) -> Result<Self> {
        // Check if Taskwarrior is installed
        let version_output = Command::new("task")
            .arg("--version")
            .output()
            .map_err(|_| Error::TaskwarriorNotFound)?;

        let version_str = String::from_utf8_lossy(&version_output.stdout);
        Self::check_version(&version_str)?;

        // Read Habitica credentials (env vars take precedence over .taskrc)
        let habitica_user_id =
            Self::get_habitica_credential("HABITICA_USER_ID", "rc.habitica.user_id")?;
        let habitica_api_key =
            Self::get_habitica_credential("HABITICA_API_KEY", "rc.habitica.api_key")?;

        // Validate credentials are present
        if habitica_user_id.is_empty() || habitica_api_key.is_empty() {
            return Err(Error::InvalidHabiticaCredentials);
        }

        // Read task note configuration
        let task_note_location =
            Self::get_taskrc_value_or_default("rc.tasknote.location", "~/.task/notes/")?;
        let task_note_dir = Self::expand_path(&task_note_location)?;

        let task_note_prefix =
            Self::get_taskrc_value_or_default("rc.tasknote.prefix", "[tasknote]")?;

        let task_note_extension =
            Self::get_taskrc_value_or_default("rc.tasknote.extension", ".txt")?;

        // Get data directory
        let data_location_str = Self::get_taskrc_value("rc.data.location")?;
        let data_location = Self::expand_path(&data_location_str)?;

        Ok(Config {
            habitica_user_id,
            habitica_api_key,
            task_note_dir,
            task_note_prefix,
            task_note_extension,
            data_location,
            verbose,
        })
    }

    /// Get the path to the stats cache file
    pub fn stats_cache_path(&self) -> PathBuf {
        self.data_location.join("cached_habitica_stats.json")
    }

    /// Check if Taskwarrior version is compatible
    fn check_version(version_str: &str) -> Result<()> {
        // Extract version number from output like "3.4.2" or "2.6.2"
        let version = version_str
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().last())
            .ok_or_else(|| Error::TaskwarriorVersionTooOld("unknown".to_string()))?;

        // Parse version components
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return Err(Error::TaskwarriorVersionTooOld(version.to_string()));
        }

        let major: u32 = parts[0].parse().unwrap_or(0);
        let minor: u32 = parts[1].parse().unwrap_or(0);

        // We need at least version 2.5.0 for import functionality
        // But we're targeting 3.4.2+
        if major < 2 || (major == 2 && minor < 5) {
            return Err(Error::TaskwarriorVersionTooOld(version.to_string()));
        }

        Ok(())
    }

    /// Get a value from Taskwarrior config using `task _get`
    fn get_taskrc_value(key: &str) -> Result<String> {
        let output = Command::new("task")
            .arg("rc.hooks=off")
            .arg("_get")
            .arg(key)
            .output()
            .map_err(|e| Error::config(format!("Failed to run task command: {}", e)))?;

        if !output.status.success() {
            return Err(Error::config(format!(
                "Failed to get config value for {}",
                key
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get a value from Taskwarrior config with a default fallback
    fn get_taskrc_value_or_default(key: &str, default: &str) -> Result<String> {
        let value = Self::get_taskrc_value(key)?;
        if value.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(value)
        }
    }

    /// Get Habitica credential from environment variable or .taskrc
    /// Environment variables take precedence over .taskrc values
    fn get_habitica_credential(env_var: &str, taskrc_key: &str) -> Result<String> {
        // Check environment variable first
        if let Ok(value) = env::var(env_var) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }

        Self::get_taskrc_value(taskrc_key)
    }

    /// Expand ~ in paths to home directory
    fn expand_path(path: &str) -> Result<PathBuf> {
        if let Some(stripped) = path.strip_prefix('~') {
            let home = dirs::home_dir()
                .ok_or_else(|| Error::config("Could not determine home directory"))?;
            let rest = stripped.strip_prefix('/').unwrap_or(stripped);
            Ok(home.join(rest))
        } else {
            Ok(PathBuf::from(path))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path() {
        // This will only work if HOME is set
        if std::env::var("HOME").is_ok() {
            let expanded =
                Config::expand_path("~/.task/notes").expect("Failed to expand path in test");
            assert!(expanded.to_string_lossy().contains(".task/notes"));
            assert!(!expanded.to_string_lossy().starts_with('~'));
        }
    }

    #[test]
    fn test_expand_path_no_tilde() {
        let path = "/tmp/test";
        let expanded = Config::expand_path(path).expect("Failed to expand path in test");
        assert_eq!(expanded.to_string_lossy(), path);
    }
}
