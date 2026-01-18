use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer};

/// Taskwarrior date format: YYYYMMDDTHHMMSSZ
const TW_DATE_FORMAT: &str = "%Y%m%dT%H%M%S";

/// Deserialize a Taskwarrior date string
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // Remove the trailing 'Z' if present
    let s = s.trim_end_matches('Z');
    NaiveDateTime::parse_from_str(s, TW_DATE_FORMAT)
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        .map_err(serde::de::Error::custom)
}

/// Deserialize an optional Taskwarrior date string
pub fn deserialize_opt<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => {
            let s = s.trim_end_matches('Z');
            NaiveDateTime::parse_from_str(s, TW_DATE_FORMAT)
                .map(|dt| Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)))
                .map_err(serde::de::Error::custom)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TestStruct {
        #[serde(deserialize_with = "super::deserialize")]
        date: DateTime<Utc>,
    }

    #[test]
    fn test_taskwarrior_date_format() {
        let json = r#"{"date":"20260118T184624Z"}"#;
        let parsed: TestStruct = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(parsed.date.format("%Y%m%dT%H%M%SZ").to_string(), "20260118T184624Z");
    }
}
