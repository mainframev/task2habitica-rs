use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    config::Config,
    error::Result,
    habitica::{HabiticaClient, StatsCache},
    sync::{ConflictResolver, ResolutionAction},
    taskwarrior::{TaskStatus, TaskwarriorClient},
};

pub fn handle_sync(config: &Config) -> Result<()> {
    let tw_client = TaskwarriorClient::new();
    let h_client = HabiticaClient::new(config)?;
    let resolver = ConflictResolver::new(config, &tw_client, &h_client);

    println!("Syncing tasks between Taskwarrior and Habitica...\n");

    // Get tasks from both sides
    let tw_only = tw_client.get_pending_without_habitica()?;
    let tw_synced = tw_client.get_tasks_with_habitica()?;
    let h_tasks = h_client.get_all_tasks()?;

    // Get current user stats
    let mut current_stats = h_client.get_user_stats()?;

    // Handle tasks that only exist in Taskwarrior
    for tw_task in tw_only {
        println!("Task: {}", tw_task.description);
        println!("    Status: Created in Taskwarrior.");
        println!("    Action: Pushing to Habitica and updating Habitica ID in Taskwarrior.");
        println!();

        let mut stats_cache = Some(StatsCache::new(current_stats.clone()));
        let updated_task = resolver.push_to_habitica(&tw_task, &mut stats_cache)?;
        tw_client.import(&updated_task)?;

        if let Some(cache) = stats_cache {
            if let Some(new_stats) = cache.current.clone() {
                current_stats = new_stats;
            }
            for msg in cache.get_diff_messages() {
                println!("    {}", msg);
            }
        }
    }

    // Create maps for efficient lookup
    let h_tasks_map: HashMap<Uuid, _> = h_tasks
        .iter()
        .filter_map(|t| t.id.map(|id| (id, t)))
        .collect();

    let tw_synced_map: HashMap<Uuid, _> = tw_synced
        .iter()
        .filter_map(|t| t.habitica_uuid.map(|id| (id, t)))
        .collect();

    // Get all unique Habitica UUIDs
    let mut all_h_uuids: Vec<Uuid> = h_tasks_map.keys().copied().collect();
    all_h_uuids.extend(tw_synced_map.keys().copied());
    all_h_uuids.sort();
    all_h_uuids.dedup();

    // Process each task
    for h_uuid in all_h_uuids {
        let h_task_opt = h_tasks_map.get(&h_uuid);
        let tw_task_opt = tw_synced_map.get(&h_uuid);

        match (h_task_opt, tw_task_opt) {
            (Some(h_task), None) => {
                // Task only exists on Habitica
                println!("Task: {}", h_task.text);
                println!("    Status: Created on Habitica.");
                println!("    Action: Importing into Taskwarrior.");
                println!();

                let tw_task = resolver.pull_from_habitica(h_task, None)?;
                tw_client.import(&tw_task)?;
            }

            (None, Some(tw_task)) => {
                // Task was deleted on Habitica
                println!("Task: {}", tw_task.description);
                println!("    Status: Deleted on Habitica.");

                if tw_task.status == TaskStatus::Completed {
                    println!("    Action: Already completed in Taskwarrior. Leaving status as Completed. Unsetting Habitica ID.");
                    let mut updated = (*tw_task).clone();
                    updated.habitica_uuid = None;
                    tw_client.import(&updated)?;
                } else {
                    println!("    Action: Setting status to Deleted in Taskwarrior. Unsetting Habitica ID.");
                    let mut updated = (*tw_task).clone();
                    updated.status = TaskStatus::Deleted;
                    updated.habitica_uuid = None;
                    tw_client.import(&updated)?;
                }
                println!();
            }

            (Some(h_task), Some(tw_task)) => {
                // Task exists on both sides
                match resolver.resolve(tw_task, h_task) {
                    ResolutionAction::NoChange => {
                        if config.verbose {
                            println!("Habitica Task:    {}", h_task.text);
                            println!("Taskwarrior Task: {}", tw_task.description);
                            println!("    Status: Exists on both Habitica and Taskwarrior.");
                            println!("    Action: Tasks are equal. Doing nothing.");
                            println!();
                        }
                    }

                    ResolutionAction::UseHabitica => {
                        println!("Habitica Task:    {}", h_task.text);
                        println!("Taskwarrior Task: {}", tw_task.description);
                        println!("    Status: Exists on both Habitica and Taskwarrior.");
                        println!("    Action: Habitica task is most recently modified. Updating in Taskwarrior.");
                        println!();

                        let updated_tw = resolver.pull_from_habitica(h_task, Some(tw_task))?;
                        tw_client.import(&updated_tw)?;
                    }

                    ResolutionAction::UseTaskwarrior => {
                        println!("Habitica Task:    {}", h_task.text);
                        println!("Taskwarrior Task: {}", tw_task.description);
                        println!("    Status: Exists on both Habitica and Taskwarrior.");
                        println!("    Action: Taskwarrior task is most recently modified. Updating on Habitica.");

                        let mut stats_cache = Some(StatsCache::new(current_stats.clone()));
                        let old_tw = resolver.pull_from_habitica(h_task, Some(tw_task))?;
                        let updated_tw =
                            resolver.modify_on_habitica(&old_tw, tw_task, &mut stats_cache)?;
                        tw_client.import(&updated_tw)?;

                        if let Some(cache) = stats_cache {
                            if let Some(new_stats) = cache.current.clone() {
                                current_stats = new_stats;
                            }
                            for msg in cache.get_diff_messages() {
                                println!("    {}", msg);
                            }
                        }
                        println!();
                    }
                }
            }

            (None, None) => {
                // This shouldn't happen since we only iterate over keys that
                // exist in at least one map, but we need to
                // handle it for completeness
            }
        }
    }

    println!("Sync complete!");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sync_command_exists() {}
}
