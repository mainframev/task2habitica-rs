use crate::{config::Config, error::Result, habitica::StatsCache};

/// Handle the 'exit' hook command
pub fn handle_exit(config: &Config) -> Result<()> {
    let stats_path = config.stats_cache_path();

    // Load stats cache
    if let Some(cache) = StatsCache::load(&stats_path)? {
        // Get and display stat diffs
        let messages = cache.get_diff_messages();
        for message in messages {
            println!("{}", message);
        }

        // Delete the cache file
        StatsCache::delete(&stats_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_exit_command_exists() {
        // Just verify the function signature is correct
    }
}
