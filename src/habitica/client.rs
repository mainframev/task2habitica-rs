use std::{thread, time::Duration};

use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{Error, Result},
    habitica::task::{HabiticaResponse, HabiticaTask, ResponseWithStats, UserStats},
};

/// Direction for scoring a task
#[derive(Debug, Clone, Copy)]
pub enum ScoreDirection {
    Up,
    Down,
}

impl ScoreDirection {
    const fn as_str(&self) -> &str {
        match self {
            ScoreDirection::Up => "up",
            ScoreDirection::Down => "down",
        }
    }
}

/// Client for interacting with the Habitica API
pub struct HabiticaClient {
    client: Client,
    base_url: String,
}

impl HabiticaClient {
    /// Create a new Habitica client with credentials from config
    pub fn new(config: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();

        headers.insert(
            "x-api-user",
            HeaderValue::from_str(&config.habitica_user_id)
                .map_err(|_| Error::InvalidHabiticaCredentials)?,
        );

        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&config.habitica_api_key)
                .map_err(|_| Error::InvalidHabiticaCredentials)?,
        );

        headers.insert(
            "x-client",
            HeaderValue::from_static("cab16cfa-e951-4dc3-a468-1abadc1dd109-Task2HabiticaRust"),
        );

        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let client = Client::builder().default_headers(headers).build()?;

        Ok(HabiticaClient {
            client,
            base_url: "https://habitica.com/api".to_string(),
        })
    }

    /// Rate limiting: wait 1 second between requests
    fn rate_limit(&self) {
        thread::sleep(Duration::from_secs(1));
    }

    /// Get all tasks of a specific type
    pub fn get_tasks(&self, task_type: Option<&str>) -> Result<Vec<HabiticaTask>> {
        self.rate_limit();

        let url = format!("{}/v3/tasks/user", self.base_url);
        let mut request = self.client.get(&url);

        if let Some(type_param) = task_type {
            request = request.query(&[("type", type_param)]);
        }

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        let api_response: HabiticaResponse<Vec<HabiticaTask>> = response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(api_response.data.unwrap_or_default())
    }

    /// Get all relevant tasks (todos, dailies, and completed todos)
    pub fn get_all_tasks(&self) -> Result<Vec<HabiticaTask>> {
        let mut tasks = Vec::new();

        // Get todos
        tasks.extend(self.get_tasks(Some("todos"))?);

        // Get dailies
        tasks.extend(self.get_tasks(Some("dailys"))?);

        // Get completed todos
        tasks.extend(self.get_tasks(Some("_allCompletedTodos"))?);

        Ok(tasks)
    }

    /// Create a new task on Habitica
    pub fn create_task(
        &self,
        task: &HabiticaTask,
    ) -> Result<(HabiticaTask, Option<UserStats>, Option<String>)> {
        self.rate_limit();

        let url = format!("{}/v3/tasks/user", self.base_url);
        let response = self.client.post(&url).json(task).send()?;

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        let api_response: HabiticaResponse<ResponseWithStats<HabiticaTask>> = response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let response_data = api_response
            .data
            .ok_or_else(|| Error::HabiticaApiError("No data in response".to_string()))?;

        let item_drop = response_data.item_drop_message();
        Ok((response_data.data, response_data.stats, item_drop))
    }

    /// Update an existing task on Habitica
    pub fn update_task(
        &self,
        task_id: Uuid,
        task: &HabiticaTask,
    ) -> Result<(HabiticaTask, Option<UserStats>, Option<String>)> {
        self.rate_limit();

        let url = format!("{}/v3/tasks/{}", self.base_url, task_id);
        let response = self.client.put(&url).json(task).send()?;

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        let api_response: HabiticaResponse<ResponseWithStats<HabiticaTask>> = response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let response_data = api_response
            .data
            .ok_or_else(|| Error::HabiticaApiError("No data in response".to_string()))?;

        let item_drop = response_data.item_drop_message();
        Ok((response_data.data, response_data.stats, item_drop))
    }

    /// Delete a task from Habitica
    pub fn delete_task(&self, task_id: Uuid) -> Result<()> {
        self.rate_limit();

        let url = format!("{}/v3/tasks/{}", self.base_url, task_id);
        let response = self.client.delete(&url).send()?;

        // Treat 404 as success - task already doesn't exist
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        let api_response: HabiticaResponse<serde_json::Value> = response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    /// Score a task (mark as complete/incomplete)
    pub fn score_task(
        &self,
        task_id: Uuid,
        direction: ScoreDirection,
    ) -> Result<(Option<UserStats>, Option<String>)> {
        self.rate_limit();

        let url = format!(
            "{}/v3/tasks/{}/score/{}",
            self.base_url,
            task_id,
            direction.as_str()
        );
        let response = self.client.post(&url).body("").send()?;

        // Treat 404 as success with no stats update - task already doesn't exist
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok((None, None));
        }

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        let api_response: HabiticaResponse<ResponseWithStats<serde_json::Value>> =
            response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let response_data = api_response
            .data
            .ok_or_else(|| Error::HabiticaApiError("No data in response".to_string()))?;

        let item_drop = response_data.item_drop_message();
        Ok((response_data.stats, item_drop))
    }

    /// Get user stats
    pub fn get_user_stats(&self) -> Result<UserStats> {
        self.rate_limit();

        let url = format!("{}/v4/user", self.base_url);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(Error::HabiticaApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct UserResponse {
            stats: UserStats,
        }

        let api_response: HabiticaResponse<UserResponse> = response.json()?;

        if !api_response.success {
            return Err(Error::HabiticaApiError(
                api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(api_response
            .data
            .ok_or_else(|| Error::HabiticaApiError("No data in response".to_string()))?
            .stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_direction() {
        assert_eq!(ScoreDirection::Up.as_str(), "up");
        assert_eq!(ScoreDirection::Down.as_str(), "down");
    }
}
