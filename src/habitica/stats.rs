use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{error::Result, habitica::task::UserStats};

/// Cache of user stats for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsCache {
    pub old: UserStats,
    pub current: Option<UserStats>,
    pub drops: Vec<String>,
}

impl StatsCache {
    /// Create a new stats cache
    pub const fn new(stats: UserStats) -> Self {
        StatsCache {
            old: stats,
            current: None,
            drops: Vec::new(),
        }
    }

    /// Update with new stats
    pub fn update(&mut self, stats: Option<UserStats>, drop_message: Option<String>) {
        if let Some(s) = stats {
            self.current = Some(s);
        }
        if let Some(msg) = drop_message {
            self.drops.push(msg);
        }
    }

    /// Load stats cache from file
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let cache: StatsCache = serde_json::from_str(&content)?;
        Ok(Some(cache))
    }

    /// Save stats cache to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Delete the cache file
    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Get a human-readable diff of stats changes
    pub fn get_diff_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();

        let new = match &self.current {
            Some(s) => s,
            None => return self.drops.clone(),
        };

        // Check for level changes
        let lvl_diff = new.lvl - self.old.lvl;
        if lvl_diff > 0 {
            messages.push(format!("LEVEL UP! ({} -> {})", self.old.lvl, new.lvl));
        } else if lvl_diff < 0 {
            messages.push(format!("LEVEL LOST! ({} -> {})", self.old.lvl, new.lvl));
        }

        // HP changes
        if let Some(msg) =
            Self::format_stat_diff("HP", self.old.hp, new.hp, new.max_hp.map(|m| m as f64))
        {
            messages.push(msg);
        }

        // MP changes
        if let Some(msg) =
            Self::format_stat_diff("MP", self.old.mp, new.mp, new.max_mp.map(|m| m as f64))
        {
            messages.push(msg);
        }

        // Exp changes (only if level didn't change)
        if lvl_diff == 0 {
            if let Some(msg) = Self::format_stat_diff(
                "Exp",
                self.old.exp,
                new.exp,
                new.to_next_level.map(|m| m as f64),
            ) {
                messages.push(msg);
            }
        }

        // Gold changes
        if let Some(msg) = Self::format_stat_diff("Gold", self.old.gp, new.gp, None) {
            messages.push(msg);
        }

        // Add item drops
        messages.extend(self.drops.clone());

        messages
    }

    /// Format a stat difference message
    fn format_stat_diff(
        name: &str,
        old_val: f64,
        new_val: f64,
        max_val: Option<f64>,
    ) -> Option<String> {
        let diff = new_val - old_val;

        if diff.abs() < 0.01 {
            return None;
        }

        let dir = if diff > 0.0 { "+" } else { "-" };
        let abs_diff = diff.abs();

        let diff_str = if abs_diff < 1.0 {
            format!("{:.2}", abs_diff)
        } else {
            format!("{}", abs_diff.round() as i32)
        };

        let new_str = if new_val < 1.0 {
            format!("{:.2}", new_val)
        } else {
            format!("{}", new_val.round() as i32)
        };

        let msg = if let Some(max) = max_val {
            format!("{}:{}{} ({}/{})", name, dir, diff_str, new_str, max as i32)
        } else {
            format!("{}:{}{} ({})", name, dir, diff_str, new_str)
        };

        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stats(hp: f64, mp: f64, exp: f64, gp: f64, lvl: i32) -> UserStats {
        UserStats {
            hp,
            max_hp: Some(50),
            mp,
            max_mp: Some(50),
            exp,
            to_next_level: Some(100),
            gp,
            lvl,
        }
    }

    #[test]
    fn test_stats_diff_no_change() {
        let stats = test_stats(50.0, 50.0, 0.0, 100.0, 1);
        let cache = StatsCache::new(stats.clone());

        let messages = cache.get_diff_messages();
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_stats_diff_with_changes() {
        let old_stats = test_stats(50.0, 50.0, 0.0, 100.0, 1);
        let new_stats = test_stats(45.0, 52.0, 10.0, 105.5, 1);

        let mut cache = StatsCache::new(old_stats);
        cache.update(Some(new_stats), None);

        let messages = cache.get_diff_messages();
        assert!(!messages.is_empty());
        assert!(messages.iter().any(|m| m.contains("HP")));
        assert!(messages.iter().any(|m| m.contains("MP")));
        assert!(messages.iter().any(|m| m.contains("Exp")));
        assert!(messages.iter().any(|m| m.contains("Gold")));
    }

    #[test]
    fn test_level_up() {
        let old_stats = test_stats(50.0, 50.0, 90.0, 100.0, 1);
        let new_stats = test_stats(50.0, 50.0, 10.0, 100.0, 2);

        let mut cache = StatsCache::new(old_stats);
        cache.update(Some(new_stats), None);

        let messages = cache.get_diff_messages();
        assert!(messages.iter().any(|m| m.contains("LEVEL UP")));
        // Exp should not be shown when level changes
        assert!(!messages.iter().any(|m| m.contains("Exp")));
    }

    #[test]
    fn test_item_drop() {
        let stats = test_stats(50.0, 50.0, 0.0, 100.0, 1);
        let mut cache = StatsCache::new(stats);
        cache.update(None, Some("You found a Sword!".to_string()));

        let messages = cache.get_diff_messages();
        assert!(messages.iter().any(|m| m.contains("Sword")));
    }
}
