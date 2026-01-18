use std::process::Command;

use crate::{
    error::{Error, Result},
    taskwarrior::task::Task,
};

/// Client for interacting with Taskwarrior
pub struct TaskwarriorClient;

impl TaskwarriorClient {
    pub const fn new() -> Self {
        TaskwarriorClient
    }

    /// Export tasks matching the given filters
    pub fn export(&self, filters: &[&str]) -> Result<Vec<Task>> {
        let mut args = vec!["rc.hooks=off"];
        args.extend(filters);
        args.push("export");

        let output = Command::new("task").args(&args).output().map_err(|e| {
            Error::TaskwarriorCommandFailed(format!("Failed to execute task export: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::TaskwarriorCommandFailed(format!(
                "task export failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Handle empty output
        if stdout.trim().is_empty() || stdout.trim() == "[]" {
            return Ok(Vec::new());
        }

        serde_json::from_str(&stdout).map_err(|e| {
            Error::TaskwarriorParseFailed(format!("Failed to parse task export JSON: {}", e))
        })
    }

    /// Import a task into Taskwarrior
    pub fn import(&self, task: &Task) -> Result<String> {
        let task_json = serde_json::to_string(task)?;

        let output = Command::new("task")
            .args(["import", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(task_json.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| {
                Error::TaskwarriorCommandFailed(format!("Failed to execute task import: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::TaskwarriorCommandFailed(format!(
                "task import failed: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get a configuration value from Taskwarrior
    pub fn get_config(&self, key: &str) -> Result<String> {
        let output = Command::new("task")
            .args(["rc.hooks=off", "_get", key])
            .output()
            .map_err(|e| {
                Error::TaskwarriorCommandFailed(format!("Failed to execute task _get: {}", e))
            })?;

        if !output.status.success() {
            return Err(Error::TaskwarriorCommandFailed(format!(
                "task _get {} failed",
                key
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get all pending tasks without Habitica UUIDs
    pub fn get_pending_without_habitica(&self) -> Result<Vec<Task>> {
        self.export(&["status:pending", "habitica_uuid.none:"])
    }

    /// Get all tasks that have Habitica UUIDs
    pub fn get_tasks_with_habitica(&self) -> Result<Vec<Task>> {
        self.export(&["habitica_uuid.any:"])
    }
}

impl Default for TaskwarriorClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Only run when taskwarrior is installed
    fn test_export_empty() {
        let client = TaskwarriorClient::new();
        // This should not error even if no tasks match
        let result = client.export(&["status:nonexistent"]);
        assert!(result.is_ok());
    }
}
