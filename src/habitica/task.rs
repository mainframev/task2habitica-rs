use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Habitica task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HabiticaTaskStatus {
    /// Task is not completed
    #[serde(rename = "pending")]
    Pending,
    /// Task is completed
    #[serde(rename = "completed")]
    Completed,
}

/// Habitica task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HabiticaTaskType {
    Todo,
    Daily,
    Habit,
    Reward,
}

/// A task as represented in the Habitica API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabiticaTask {
    /// Habitica's UUID for the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    /// Task text/title
    pub text: String,

    /// Task notes/description
    #[serde(default)]
    pub notes: String,

    /// Task type
    #[serde(rename = "type")]
    pub task_type: HabiticaTaskType,

    /// Priority (0.1=trivial, 1=easy, 1.5=medium, 2=hard)
    pub priority: f64,

    /// Whether the task is completed
    #[serde(default)]
    pub completed: bool,

    /// Due date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime<Utc>>,

    /// Last update timestamp
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// For dailies: whether the task is due today
    #[serde(rename = "isDue", default, skip_serializing)]
    pub is_due: bool,
}

impl HabiticaTask {
    /// Get the effective status based on completion and daily due status
    pub fn effective_status(&self) -> HabiticaTaskStatus {
        if self.task_type == HabiticaTaskType::Daily {
            // Dailies are pending only if they're not completed AND are due
            if !self.completed && self.is_due {
                HabiticaTaskStatus::Pending
            } else {
                HabiticaTaskStatus::Completed
            }
        } else if self.completed {
            HabiticaTaskStatus::Completed
        } else {
            HabiticaTaskStatus::Pending
        }
    }

    /// Get modification time, defaulting to now if not set
    pub fn modified_or_now(&self) -> DateTime<Utc> {
        self.updated_at.unwrap_or_else(Utc::now)
    }
}

/// Habitica API response wrapper
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct HabiticaResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// User stats from Habitica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub hp: f64,
    #[serde(rename = "maxHealth")]
    pub max_hp: Option<i32>,
    pub mp: f64,
    #[serde(rename = "maxMP")]
    pub max_mp: Option<i32>,
    pub exp: f64,
    #[serde(rename = "toNextLevel")]
    pub to_next_level: Option<i32>,
    pub gp: f64,
    pub lvl: i32,
}

/// Item drop information
#[derive(Debug, Clone, Deserialize)]
pub struct ItemDrop {
    #[serde(rename = "_tmp")]
    pub tmp: Option<ItemDropTemp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDropTemp {
    pub drop: Option<ItemDropData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDropData {
    pub dialog: Option<String>,
}

impl ItemDrop {
    pub fn message(&self) -> Option<String> {
        self.tmp
            .as_ref()
            .and_then(|t| t.drop.as_ref())
            .and_then(|d| d.dialog.clone())
    }
}

/// Response data with stats and drops
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct ResponseWithStats<T> {
    #[serde(flatten)]
    pub data: T,
    #[serde(default)]
    pub stats: Option<UserStats>,
    #[serde(rename = "_tmp", default)]
    pub tmp: Option<ItemDropTemp>,
}

impl<T> ResponseWithStats<T> {
    pub fn item_drop_message(&self) -> Option<String> {
        self.tmp
            .as_ref()
            .and_then(|t| t.drop.as_ref())
            .and_then(|d| d.dialog.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_habitica_task_effective_status() {
        let mut task = HabiticaTask {
            id: None,
            text: "Test".to_string(),
            notes: String::new(),
            task_type: HabiticaTaskType::Todo,
            priority: 1.0,
            completed: false,
            date: None,
            updated_at: None,
            is_due: false,
        };

        // Todo not completed should be pending
        assert_eq!(task.effective_status(), HabiticaTaskStatus::Pending);

        // Todo completed should be completed
        task.completed = true;
        assert_eq!(task.effective_status(), HabiticaTaskStatus::Completed);

        // Daily not due should be completed even if not completed
        task.task_type = HabiticaTaskType::Daily;
        task.completed = false;
        task.is_due = false;
        assert_eq!(task.effective_status(), HabiticaTaskStatus::Completed);

        // Daily due and not completed should be pending
        task.is_due = true;
        assert_eq!(task.effective_status(), HabiticaTaskStatus::Pending);
    }
}
