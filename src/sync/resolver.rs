use crate::{
    config::Config,
    error::Result,
    habitica::{HabiticaClient, HabiticaTask, ScoreDirection, StatsCache},
    sync::converter,
    taskwarrior::{NotesManager, Task, TaskwarriorClient},
};

/// Result of resolving a conflict between Taskwarrior and Habitica
pub enum ResolutionAction {
    /// Keep Taskwarrior version and push to Habitica
    UseTaskwarrior,
    /// Keep Habitica version and update Taskwarrior
    UseHabitica,
    /// Tasks are equivalent, no action needed
    NoChange,
}

/// Resolve conflicts between Taskwarrior and Habitica tasks
#[allow(dead_code)]
pub struct ConflictResolver<'a> {
    config: &'a Config,
    tw_client: &'a TaskwarriorClient,
    h_client: &'a HabiticaClient,
    notes_manager: NotesManager<'a>,
}

impl<'a> ConflictResolver<'a> {
    pub const fn new(
        config: &'a Config,
        tw_client: &'a TaskwarriorClient,
        h_client: &'a HabiticaClient,
    ) -> Self {
        ConflictResolver {
            config,
            tw_client,
            h_client,
            notes_manager: NotesManager::new(config),
        }
    }

    /// Determine which version of a task should win based on modification time
    pub fn resolve(&self, tw_task: &Task, h_task: &HabiticaTask) -> ResolutionAction {
        // First check if tasks are equivalent
        if converter::tasks_are_equivalent(tw_task, h_task) {
            return ResolutionAction::NoChange;
        }

        // Compare modification times
        let tw_modified = tw_task.modified_or_now();
        let h_modified = h_task.modified_or_now();

        if h_modified > tw_modified {
            ResolutionAction::UseHabitica
        } else {
            ResolutionAction::UseTaskwarrior
        }
    }

    /// Push a Taskwarrior task to Habitica and handle scoring if needed
    pub fn push_to_habitica(
        &self,
        tw_task: &Task,
        stats_cache: &mut Option<StatsCache>,
    ) -> Result<Task> {
        // Read note content
        let note_content = self.notes_manager.read_note(tw_task)?;

        // Convert to Habitica task
        let h_task_opt = converter::taskwarrior_to_habitica(tw_task, note_content.as_deref())?;

        let Some(h_task) = h_task_opt else {
            // Task should not be synced to Habitica
            return Ok(tw_task.clone());
        };

        let mut updated_tw_task = tw_task.clone();

        // Create or update on Habitica
        let (returned_h_task, new_stats, drop_msg) = if let Some(h_id) = h_task.id {
            self.h_client.update_task(h_id, &h_task)?
        } else {
            self.h_client.create_task(&h_task)?
        };

        // Update the Habitica UUID in Taskwarrior task
        updated_tw_task.habitica_uuid = returned_h_task.id;

        // Update stats cache
        if let Some(cache) = stats_cache {
            cache.update(new_stats, drop_msg.clone());
        }

        // If task is already completed, score it
        if tw_task.status.is_completed() && returned_h_task.id.is_some() {
            if let Some(h_id) = returned_h_task.id {
                let (score_stats, score_drop) =
                    self.h_client.score_task(h_id, ScoreDirection::Up)?;
                if let Some(cache) = stats_cache {
                    cache.update(score_stats, score_drop);
                }
            }
        }

        Ok(updated_tw_task)
    }

    /// Update Taskwarrior from Habitica task
    pub fn pull_from_habitica(
        &self,
        h_task: &HabiticaTask,
        existing_tw: Option<&Task>,
    ) -> Result<Task> {
        // Convert to Taskwarrior task
        let mut tw_task = converter::habitica_to_taskwarrior(h_task, existing_tw)?;

        // Import note from Habitica
        self.notes_manager
            .import_note_from_habitica(&mut tw_task, &h_task.notes)?;

        Ok(tw_task)
    }

    /// Handle status transitions that require scoring
    pub fn handle_status_change(
        &self,
        old_tw: &Task,
        new_tw: &Task,
        stats_cache: &mut Option<StatsCache>,
    ) -> Result<Task> {
        let old_status = old_tw.status;
        let new_status = new_tw.status;

        // Check if we need to score on Habitica
        let score_direction = match (old_status.is_completed(), new_status.is_completed()) {
            (false, true) => Some(ScoreDirection::Up), // Pending -> Completed
            (true, false) => Some(ScoreDirection::Down), // Completed -> Pending
            _ => None,
        };

        if let (Some(direction), Some(h_id)) = (score_direction, new_tw.habitica_uuid) {
            let (new_stats, drop_msg) = self.h_client.score_task(h_id, direction)?;
            if let Some(cache) = stats_cache {
                cache.update(new_stats, drop_msg);
            }
        }

        Ok(new_tw.clone())
    }

    /// Modify a task on Habitica based on changes from Taskwarrior
    pub fn modify_on_habitica(
        &self,
        old_tw: &Task,
        new_tw: &Task,
        stats_cache: &mut Option<StatsCache>,
    ) -> Result<Task> {
        // Check if task should be deleted from Habitica
        if !new_tw.status.should_sync_to_habitica() && old_tw.habitica_uuid.is_some() {
            if let Some(h_id) = old_tw.habitica_uuid {
                self.h_client.delete_task(h_id)?;
            }
            let mut updated = new_tw.clone();
            updated.habitica_uuid = None;
            return Ok(updated);
        }

        // Check if task should be created on Habitica
        if new_tw.status.should_sync_to_habitica() && !old_tw.status.should_sync_to_habitica() {
            return self.push_to_habitica(new_tw, stats_cache);
        }

        // Check if we need to push changes
        let note_content = self.notes_manager.read_note(new_tw)?;
        let new_h_opt = converter::taskwarrior_to_habitica(new_tw, note_content.as_deref())?;

        if let Some(new_h) = new_h_opt {
            // Update details if changed
            if let Some(h_id) = new_h.id {
                let (_, new_stats, drop_msg) = self.h_client.update_task(h_id, &new_h)?;
                if let Some(cache) = stats_cache {
                    cache.update(new_stats, drop_msg);
                }
            }

            // Handle status changes (scoring)
            return self.handle_status_change(old_tw, new_tw, stats_cache);
        }

        Ok(new_tw.clone())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_action() {
        // Just test that the enum exists and can be constructed
        let _action = ResolutionAction::NoChange;
        let _action = ResolutionAction::UseTaskwarrior;
        let _action = ResolutionAction::UseHabitica;
    }
}
