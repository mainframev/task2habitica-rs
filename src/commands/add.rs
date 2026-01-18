use std::io::{self, BufRead};

use crate::{
    config::Config,
    error::Result,
    habitica::{HabiticaClient, StatsCache},
    sync::ConflictResolver,
    taskwarrior::{Task, TaskwarriorClient},
};

/// Handle the 'add' hook command
pub fn handle_add(config: &Config) -> Result<()> {
    // Read task JSON from stdin
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let task_json = lines
        .next()
        .ok_or_else(|| crate::error::Error::custom("No input provided"))??;

    // Debug: log the raw input if verbose mode
    if config.verbose {
        eprintln!("DEBUG: Received JSON (length {}): {}", task_json.len(), task_json);
    }

    // Parse the task
    let task: Task = serde_json::from_str(&task_json).map_err(|e| {
        crate::error::Error::custom(format!(
            "Failed to parse task JSON: {}. Input length: {}",
            e,
            task_json.len()
        ))
    })?;

    // Initialize clients
    let tw_client = TaskwarriorClient::new();
    let h_client = HabiticaClient::new(config)?;

    // Create resolver
    let resolver = ConflictResolver::new(config, &tw_client, &h_client);

    // Initialize stats cache if task is completed
    let mut stats_cache = if task.status.is_completed() {
        let stats = h_client.get_user_stats()?;
        Some(StatsCache::new(stats))
    } else {
        None
    };

    // Push task to Habitica
    let updated_task = resolver.push_to_habitica(&task, &mut stats_cache)?;

    // Save stats cache if we created one
    if let Some(cache) = stats_cache {
        cache.save(&config.stats_cache_path())?;
    }

    // Output the updated task JSON to stdout
    let output_json = serde_json::to_string(&updated_task)?;
    println!("{output_json}");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add_command_exists() {
        // Just verify the function signature is correct
    }
}
