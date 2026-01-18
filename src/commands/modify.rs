use std::io::{self, BufRead};

use crate::{
    config::Config,
    error::Result,
    habitica::{HabiticaClient, StatsCache},
    sync::{converter, ConflictResolver},
    taskwarrior::{NotesManager, Task, TaskwarriorClient},
};

/// Handle the 'modify' hook command
pub fn handle_modify(config: &Config) -> Result<()> {
    // Read old and new task JSON from stdin
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let old_task_json = lines
        .next()
        .ok_or_else(|| crate::error::Error::custom("No old task provided"))??;
    let new_task_json = lines
        .next()
        .ok_or_else(|| crate::error::Error::custom("No new task provided"))??;

    let old_task: Task = serde_json::from_str(&old_task_json)?;
    let new_task: Task = serde_json::from_str(&new_task_json)?;

    // Check if note was recently modified
    let notes_manager = NotesManager::new(config);
    let note_recently_changed = notes_manager.note_recently_modified(&new_task)?;

    // Check if note annotations changed
    let old_note_annos = old_task.filter_note_annotations(&config.task_note_prefix);
    let new_note_annos = new_task.filter_note_annotations(&config.task_note_prefix);

    // Read note content
    let note_content = notes_manager.read_note(&new_task)?;

    // Convert both to Habitica format to compare
    let old_h_opt = converter::taskwarrior_to_habitica(&old_task, note_content.as_deref())?;
    let new_h_opt = converter::taskwarrior_to_habitica(&new_task, note_content.as_deref())?;

    // If tasks are equivalent and note hasn't changed, just output the new task
    if old_h_opt == new_h_opt && !note_recently_changed && old_note_annos == new_note_annos {
        let output_json = serde_json::to_string(&new_task)?;
        println!("{}", output_json);
        return Ok(());
    }

    // Tasks have changed, so we need to sync
    let tw_client = TaskwarriorClient::new();
    let h_client = HabiticaClient::new(config)?;
    let resolver = ConflictResolver::new(config, &tw_client, &h_client);

    // Load or create stats cache
    let mut stats_cache = StatsCache::load(&config.stats_cache_path())?
        .or_else(|| h_client.get_user_stats().ok().map(StatsCache::new));

    // Modify task on Habitica
    let updated_task = resolver.modify_on_habitica(&old_task, &new_task, &mut stats_cache)?;

    // Save stats cache
    if let Some(cache) = &stats_cache {
        cache.save(&config.stats_cache_path())?;
    }

    // Output the updated task JSON to stdout
    let output_json = serde_json::to_string(&updated_task)?;
    println!("{}", output_json);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_modify_command_exists() {}
}
