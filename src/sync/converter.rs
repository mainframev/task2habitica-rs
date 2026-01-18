use crate::{
    error::Result,
    habitica::{HabiticaTask, HabiticaTaskStatus, HabiticaTaskType},
    taskwarrior::{Task, TaskDifficulty, TaskStatus, TaskType},
};

/// Convert a Taskwarrior task to a Habitica task
pub fn taskwarrior_to_habitica(
    tw_task: &Task,
    note_content: Option<&str>,
) -> Result<Option<HabiticaTask>> {
    // Don't sync recurring or deleted tasks to Habitica
    if !tw_task.status.should_sync_to_habitica() {
        return Ok(None);
    }

    // Convert status
    let (completed, _status) = match tw_task.status {
        TaskStatus::Pending | TaskStatus::Waiting => (false, HabiticaTaskStatus::Pending),
        TaskStatus::Completed => (true, HabiticaTaskStatus::Completed),
        TaskStatus::Deleted | TaskStatus::Recurring => return Ok(None),
    };

    // Convert task type
    let task_type = match tw_task.task_type() {
        TaskType::Todo => HabiticaTaskType::Todo,
        TaskType::Daily => HabiticaTaskType::Daily,
        _ => HabiticaTaskType::Todo, // Default to todo for habits/rewards
    };

    Ok(Some(HabiticaTask {
        id: tw_task.habitica_uuid,
        text: tw_task.description.clone(),
        notes: note_content.unwrap_or("").to_string(),
        task_type,
        priority: tw_task.difficulty().to_habitica_priority(),
        completed,
        date: tw_task.due,
        updated_at: tw_task.modified,
        is_due: false, // This will be set by Habitica
    }))
}

/// Convert a Habitica task to a Taskwarrior task
pub fn habitica_to_taskwarrior(
    h_task: &HabiticaTask,
    existing_tw_task: Option<&Task>,
) -> Result<Task> {
    // Convert status
    let status = match h_task.effective_status() {
        HabiticaTaskStatus::Pending => TaskStatus::Pending,
        HabiticaTaskStatus::Completed => TaskStatus::Completed,
    };

    // Convert difficulty
    let difficulty = TaskDifficulty::from_habitica_priority(h_task.priority);

    // Convert task type
    let task_type = match h_task.task_type {
        HabiticaTaskType::Todo => TaskType::Todo,
        HabiticaTaskType::Daily => TaskType::Daily,
        HabiticaTaskType::Habit => TaskType::Habit,
        HabiticaTaskType::Reward => TaskType::Reward,
    };

    // If we have an existing task, preserve its UUID and extra fields
    let (uuid, extra, annotations) = if let Some(existing) = existing_tw_task {
        (
            existing.uuid,
            existing.extra.clone(),
            existing.annotations.clone(),
        )
    } else {
        (uuid::Uuid::new_v4(), serde_json::Map::new(), None)
    };

    Ok(Task {
        uuid,
        description: h_task.text.clone(),
        status,
        modified: h_task.updated_at,
        due: h_task.date,
        annotations,
        habitica_uuid: h_task.id,
        habitica_difficulty: Some(difficulty),
        habitica_task_type: Some(task_type),
        extra,
    })
}

/// Update a Taskwarrior task with data from a Habitica task
/// Preserves Taskwarrior-specific fields like UUID, annotations, etc.
pub fn update_taskwarrior_from_habitica(tw_task: &mut Task, h_task: &HabiticaTask) -> Result<()> {
    // Update fields from Habitica
    tw_task.description = h_task.text.clone();
    tw_task.due = h_task.date;
    tw_task.modified = h_task.updated_at;
    tw_task.habitica_uuid = h_task.id;
    tw_task.habitica_difficulty = Some(TaskDifficulty::from_habitica_priority(h_task.priority));

    let task_type = match h_task.task_type {
        HabiticaTaskType::Todo => TaskType::Todo,
        HabiticaTaskType::Daily => TaskType::Daily,
        HabiticaTaskType::Habit => TaskType::Habit,
        HabiticaTaskType::Reward => TaskType::Reward,
    };
    tw_task.habitica_task_type = Some(task_type);

    // Update status, but preserve Waiting status from Taskwarrior
    tw_task.status = match (h_task.effective_status(), tw_task.status) {
        (HabiticaTaskStatus::Pending, TaskStatus::Waiting) => TaskStatus::Waiting,
        (HabiticaTaskStatus::Pending, _) => TaskStatus::Pending,
        (HabiticaTaskStatus::Completed, _) => TaskStatus::Completed,
    };

    Ok(())
}

/// Check if two tasks are equivalent (ignoring modification time)
pub fn tasks_are_equivalent(tw_task: &Task, h_task: &HabiticaTask) -> bool {
    // Check basic fields
    if tw_task.description != h_task.text {
        return false;
    }

    if tw_task.due != h_task.date {
        return false;
    }

    // Check difficulty
    if tw_task.difficulty().to_habitica_priority() != h_task.priority {
        return false;
    }

    // Check task type
    let tw_type = match tw_task.task_type() {
        TaskType::Todo => HabiticaTaskType::Todo,
        TaskType::Daily => HabiticaTaskType::Daily,
        TaskType::Habit => HabiticaTaskType::Habit,
        TaskType::Reward => HabiticaTaskType::Reward,
    };
    if tw_type != h_task.task_type {
        return false;
    }

    // Check status
    let tw_completed = tw_task.status.is_completed();
    if tw_completed != h_task.completed {
        return false;
    }

    // Check Habitica UUID
    if tw_task.habitica_uuid != h_task.id {
        return false;
    }

    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::bool_assert_comparison)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn test_tw_task() -> Task {
        Task {
            uuid: uuid::Uuid::new_v4(),
            description: "Test task".to_string(),
            status: TaskStatus::Pending,
            modified: Some(Utc::now()),
            due: None,
            annotations: None,
            habitica_uuid: Some(uuid::Uuid::new_v4()),
            habitica_difficulty: Some(TaskDifficulty::Easy),
            habitica_task_type: Some(TaskType::Todo),
            extra: serde_json::Map::new(),
        }
    }

    fn test_h_task() -> HabiticaTask {
        HabiticaTask {
            id: Some(uuid::Uuid::new_v4()),
            text: "Test task".to_string(),
            notes: String::new(),
            task_type: HabiticaTaskType::Todo,
            priority: 1.0,
            completed: false,
            date: None,
            updated_at: Some(Utc::now()),
            is_due: false,
        }
    }

    #[test]
    fn test_taskwarrior_to_habitica_pending() {
        let tw_task = test_tw_task();
        let h_task = taskwarrior_to_habitica(&tw_task, None).unwrap().unwrap();

        assert_eq!(h_task.text, tw_task.description);
        assert_eq!(h_task.completed, false);
        assert_eq!(h_task.priority, 1.0);
    }

    #[test]
    fn test_taskwarrior_to_habitica_completed() {
        let mut tw_task = test_tw_task();
        tw_task.status = TaskStatus::Completed;

        let h_task = taskwarrior_to_habitica(&tw_task, None).unwrap().unwrap();
        assert_eq!(h_task.completed, true);
    }

    #[test]
    fn test_taskwarrior_to_habitica_deleted() {
        let mut tw_task = test_tw_task();
        tw_task.status = TaskStatus::Deleted;

        let result = taskwarrior_to_habitica(&tw_task, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_habitica_to_taskwarrior() {
        let h_task = test_h_task();
        let tw_task = habitica_to_taskwarrior(&h_task, None).unwrap();

        assert_eq!(tw_task.description, h_task.text);
        assert_eq!(tw_task.status, TaskStatus::Pending);
        assert_eq!(tw_task.habitica_uuid, h_task.id);
    }

    #[test]
    fn test_tasks_are_equivalent() {
        let tw_task = test_tw_task();
        let h_task = taskwarrior_to_habitica(&tw_task, None).unwrap().unwrap();

        assert!(tasks_are_equivalent(&tw_task, &h_task));
    }

    #[test]
    fn test_tasks_not_equivalent_different_text() {
        let tw_task = test_tw_task();
        let mut h_task = taskwarrior_to_habitica(&tw_task, None).unwrap().unwrap();
        h_task.text = "Different text".to_string();

        assert!(!tasks_are_equivalent(&tw_task, &h_task));
    }
}
