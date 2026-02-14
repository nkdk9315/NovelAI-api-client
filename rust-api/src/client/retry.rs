use crate::client::Logger;
use crate::error::{NovelAIError, Result};

const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 1000;

/// Execute an HTTP request with exponential backoff retry.
///
/// Retries on:
/// - 429 (Too Many Requests / Concurrent generation locked)
/// - Network errors (timeout, connection refused, DNS failure)
///
/// Does NOT retry on other HTTP errors (400, 401, 500, etc.).
pub async fn fetch_with_retry(
    client: &reqwest::Client,
    url: &str,
    method: reqwest::Method,
    body: Option<String>,
    api_key: &str,
    operation_name: &str,
    logger: &dyn Logger,
) -> Result<reqwest::Response> {
    for attempt in 0..=MAX_RETRIES {
        let mut request = client.request(method.clone(), url).header(
            "Authorization",
            format!("Bearer {}", api_key),
        );

        // Set content-type for POST requests with body
        if let Some(ref body_str) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(body_str.clone());
        } else {
            request = request.header("Accept", "application/json");
        }

        let response = match request.send().await {
            Ok(resp) => resp,
            Err(err) => {
                // Network error: timeout, connection refused, DNS failure, etc.
                let is_retryable = err.is_timeout()
                    || err.is_connect()
                    || err.is_request();

                if is_retryable && attempt < MAX_RETRIES {
                    let delay = retry_delay(attempt);
                    logger.warn(&format!(
                        "[NovelAI] {}: Network error ({}). Retrying in {}ms... (attempt {}/{})",
                        operation_name, err, delay, attempt + 1, MAX_RETRIES
                    ));
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }

                return Err(NovelAIError::Other(format!(
                    "{} failed: {}", operation_name, err
                )));
            }
        };

        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status().as_u16();

        // Handle 429 (rate limit / concurrent lock)
        if status == 429 {
            if attempt < MAX_RETRIES {
                let delay = retry_delay(attempt);
                logger.warn(&format!(
                    "[NovelAI] {}: Rate limited (429). Retrying in {}ms... (attempt {}/{})",
                    operation_name, delay, attempt + 1, MAX_RETRIES
                ));
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }

            // Max retries exhausted
            let text = response.text().await.unwrap_or_default();
            let sanitized = truncate_text(&text, 200);
            logger.error(&format!(
                "[NovelAI] {} error after {} retries ({}): {}",
                operation_name, MAX_RETRIES, status, sanitized
            ));
            return Err(NovelAIError::Api {
                status_code: status,
                message: format!(
                    "{} failed after {} retries: {} Too Many Requests",
                    operation_name, MAX_RETRIES, status
                ),
            });
        }

        // Other HTTP errors - don't retry
        let text = response.text().await.unwrap_or_default();
        let sanitized = truncate_text(&text, 200);
        logger.error(&format!(
            "[NovelAI] {} error ({}): {}",
            operation_name, status, sanitized
        ));
        return Err(NovelAIError::Api {
            status_code: status,
            message: format!("{} failed: {}", operation_name, status),
        });
    }

    Err(NovelAIError::Other(format!(
        "{} failed: Unknown error after {} retries",
        operation_name, MAX_RETRIES
    )))
}

/// Calculate retry delay with exponential backoff and jitter.
/// Formula: baseDelay * 2^attempt * (1 + random * 0.3)
fn retry_delay(attempt: u32) -> u64 {
    let base = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
    let jitter = 1.0 + rand::random::<f64>() * 0.3;
    (base as f64 * jitter).round() as u64
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...[truncated]", &text[..max_len])
    } else {
        text.to_string()
    }
}
