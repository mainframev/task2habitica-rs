use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Status of a Taskwarrior task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Waiting,
    Completed,
    Deleted,
    Recurring,
}

impl TaskStatus {
    /// Check if this status should be synced to Habitica
    pub const fn should_sync_to_habitica(&self) -> bool {
        matches!(
            self,
            TaskStatus::Pending | TaskStatus::Waiting | TaskStatus::Completed
        )
    }

    /// Check if this is a completed status
    pub fn is_completed(&self) -> bool {
        *self == TaskStatus::Completed
    }

    /// Check if this is a pending-like status
    pub const fn is_pending(&self) -> bool {
        matches!(self, TaskStatus::Pending | TaskStatus::Waiting)
    }
}

/// Task difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskDifficulty {
    Trivial,
    #[default]
    Easy,
    Medium,
    Hard,
}

impl TaskDifficulty {
    /// Convert to Habitica priority value
    pub const fn to_habitica_priority(&self) -> f64 {
        match self {
            TaskDifficulty::Trivial => 0.1,
            TaskDifficulty::Easy => 1.0,
            TaskDifficulty::Medium => 1.5,
            TaskDifficulty::Hard => 2.0,
        }
    }

    /// Convert from Habitica priority value
    pub fn from_habitica_priority(priority: f64) -> Self {
        if (priority - 0.1).abs() < 0.01 {
            TaskDifficulty::Trivial
        } else if (priority - 1.0).abs() < 0.01 {
            TaskDifficulty::Easy
        } else if (priority - 1.5).abs() < 0.01 {
            TaskDifficulty::Medium
        } else {
            TaskDifficulty::Hard
        }
    }
}

/// Task type (Habitica classification)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[default]
    Todo,
    Daily,
    #[serde(rename = "habit")]
    Habit,
    #[serde(rename = "reward")]
    Reward,
}

/// Annotation on a task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub entry: String,
    pub description: String,
}

/// A Taskwarrior task with all its fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub uuid: Uuid,
    pub description: String,
    pub status: TaskStatus,

    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::date_format::deserialize_opt",
        default
    )]
    pub modified: Option<DateTime<Utc>>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::date_format::deserialize_opt",
        default
    )]
    pub due: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,

    // Habitica-specific UDAs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub habitica_uuid: Option<Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub habitica_difficulty: Option<TaskDifficulty>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub habitica_task_type: Option<TaskType>,

    // Store any additional fields we don't explicitly handle
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl Task {
    /// Get the modification time, defaulting to now if not set
    pub fn modified_or_now(&self) -> DateTime<Utc> {
        self.modified.unwrap_or_else(Utc::now)
    }

    /// Get task difficulty with default
    pub fn difficulty(&self) -> TaskDifficulty {
        self.habitica_difficulty.unwrap_or_default()
    }

    /// Get task type with default
    pub fn task_type(&self) -> TaskType {
        self.habitica_task_type.unwrap_or_default()
    }

    /// Check if task has a note (based on note prefix in annotations)
    pub fn has_note_annotation(&self, note_prefix: &str) -> bool {
        self.annotations
            .as_ref()
            .is_some_and(|annos| {
                annos
                    .iter()
                    .any(|anno| anno.description.trim().starts_with(note_prefix))
            })
    }

    /// Filter annotations to only keep non-note annotations
    pub fn filter_note_annotations(&self, note_prefix: &str) -> Vec<Annotation> {
        self.annotations
            .as_ref()
            .map(|annos| {
                annos
                    .iter()
                    .filter(|anno| !anno.description.trim().starts_with(note_prefix))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        // Compare tasks ignoring modified time and extra fields
        self.uuid == other.uuid
            && self.description == other.description
            && self.status == other.status
            && self.due == other.due
            && self.habitica_uuid == other.habitica_uuid
            && self.habitica_difficulty == other.habitica_difficulty
            && self.habitica_task_type == other.habitica_task_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_difficulty_conversion() {
        assert_eq!(TaskDifficulty::Trivial.to_habitica_priority(), 0.1);
        assert_eq!(TaskDifficulty::Easy.to_habitica_priority(), 1.0);
        assert_eq!(TaskDifficulty::Medium.to_habitica_priority(), 1.5);
        assert_eq!(TaskDifficulty::Hard.to_habitica_priority(), 2.0);

        assert_eq!(
            TaskDifficulty::from_habitica_priority(0.1),
            TaskDifficulty::Trivial
        );
        assert_eq!(
            TaskDifficulty::from_habitica_priority(1.0),
            TaskDifficulty::Easy
        );
        assert_eq!(
            TaskDifficulty::from_habitica_priority(1.5),
            TaskDifficulty::Medium
        );
        assert_eq!(
            TaskDifficulty::from_habitica_priority(2.0),
            TaskDifficulty::Hard
        );
    }

    #[test]
    fn test_task_status_sync() {
        assert!(TaskStatus::Pending.should_sync_to_habitica());
        assert!(TaskStatus::Waiting.should_sync_to_habitica());
        assert!(TaskStatus::Completed.should_sync_to_habitica());
        assert!(!TaskStatus::Deleted.should_sync_to_habitica());
        assert!(!TaskStatus::Recurring.should_sync_to_habitica());
    }
}
