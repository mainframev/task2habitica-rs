use std::{fs, path::PathBuf};

use chrono::Utc;

use crate::{
    config::Config,
    error::Result,
    taskwarrior::task::{Annotation, Task},
};

/// Manages task notes stored as separate files
pub struct NotesManager<'a> {
    config: &'a Config,
}

impl<'a> NotesManager<'a> {
    pub const fn new(config: &'a Config) -> Self {
        NotesManager { config }
    }

    /// Get the path to a task's note file
    pub fn note_path(&self, task: &Task) -> PathBuf {
        self.config
            .task_note_dir
            .join(format!("{}{}", task.uuid, self.config.task_note_extension))
    }

    /// Read the note content for a task
    pub fn read_note(&self, task: &Task) -> Result<Option<String>> {
        let path = self.note_path(task);
        if path.exists() {
            Ok(Some(fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    /// Write note content for a task
    pub fn write_note(&self, task: &Task, content: &str) -> Result<()> {
        // Create notes directory if it doesn't exist
        fs::create_dir_all(&self.config.task_note_dir)?;

        let path = self.note_path(task);
        fs::write(path, content)?;
        Ok(())
    }

    /// Delete a task's note file
    pub fn delete_note(&self, task: &Task) -> Result<()> {
        let path = self.note_path(task);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if a note file was recently modified (within 60 seconds)
    pub fn note_recently_modified(&self, task: &Task) -> Result<bool> {
        let path = self.note_path(task);
        if !path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;
        let now = std::time::SystemTime::now();

        if let Ok(duration) = now.duration_since(modified) {
            Ok(duration.as_secs() <= 60)
        } else {
            Ok(false)
        }
    }

    /// Update task annotations based on note content
    /// Returns a new task with updated annotations
    pub fn sync_note_to_annotation(
        &self,
        task: &mut Task,
        note_content: Option<String>,
    ) -> Result<()> {
        // Remove existing note annotations
        let mut annotations = task.filter_note_annotations(&self.config.task_note_prefix);

        match note_content {
            Some(content) if !content.trim().is_empty() => {
                // Add new note annotation with first line as preview
                let first_line = content.lines().next().unwrap_or("").trim();
                if !first_line.is_empty() {
                    let note_annotation = Annotation {
                        entry: Utc::now().format("%Y%m%dT%H%M%SZ").to_string(),
                        description: format!("{} {}", self.config.task_note_prefix, first_line),
                    };
                    annotations.insert(0, note_annotation);
                }
            }
            _ => {
                // No note or empty note - just keep non-note annotations
            }
        }

        task.annotations = if annotations.is_empty() {
            None
        } else {
            Some(annotations)
        };

        Ok(())
    }

    /// Extract note content from Habitica task notes field
    /// and save it as a file, updating task annotations
    pub fn import_note_from_habitica(&self, task: &mut Task, note_content: &str) -> Result<()> {
        if note_content.trim().is_empty() {
            // Empty note - delete file if exists
            self.delete_note(task)?;
            self.sync_note_to_annotation(task, None)?;
        } else {
            // Write note file
            self.write_note(task, note_content)?;
            self.sync_note_to_annotation(task, Some(note_content.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn test_config() -> Config {
        Config {
            habitica_user_id: String::new(),
            habitica_api_key: String::new(),
            task_note_dir: std::env::temp_dir().join("test_notes"),
            task_note_prefix: "[tasknote]".to_string(),
            task_note_extension: ".txt".to_string(),
            data_location: std::env::temp_dir(),
            verbose: false,
        }
    }

    fn test_task() -> Task {
        Task {
            uuid: Uuid::new_v4(),
            description: "Test task".to_string(),
            status: crate::taskwarrior::task::TaskStatus::Pending,
            modified: None,
            due: None,
            annotations: None,
            habitica_uuid: None,
            habitica_difficulty: None,
            habitica_task_type: None,
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn test_note_path() {
        let config = test_config();
        let manager = NotesManager::new(&config);
        let task = test_task();

        let path = manager.note_path(&task);
        assert!(path.to_string_lossy().contains(&task.uuid.to_string()));
        assert!(path.to_string_lossy().ends_with(".txt"));
    }

    #[test]
    fn test_write_and_read_note() {
        let config = test_config();
        let manager = NotesManager::new(&config);
        let task = test_task();

        let content = "This is a test note";
        manager.write_note(&task, content).unwrap();

        let read_content = manager.read_note(&task).unwrap();
        assert_eq!(read_content, Some(content.to_string()));

        // Cleanup
        manager.delete_note(&task).unwrap();
    }
}
