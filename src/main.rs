//! # RustyIP
//!
//! RustyIP is a command-line application that periodically updates a dynamic DNS record.
//! It retrieves the current public IP address, generates a payload using a token and key,
//! and sends this payload to a specified endpoint.

mod client;
mod core;
mod utils;

use reqwest::{Client, Url};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

use crate::client::AppClient; // For client.post_form()
use crate::core::generate_payload;
use crate::utils::{retry, RETRY_DELAY};

/// The main entry point for the RustyIP application.
///
/// This function performs the following steps in a loop:
/// 1. Retrieves necessary configuration (KEY, TOKEN, HASH, HOST, SLEEP_DURATION) from environment variables.
/// 2. Constructs the target endpoint URL.
/// 3. Creates an HTTP client.
/// 4. In each iteration:
///    a. Generates a payload using `generate_payload`.
///    b. Sends the payload to the endpoint using `client.post_form`, with a retry mechanism.
///    c. Sleeps for the configured duration before the next iteration.
///
/// # Panics
///
/// This function will panic if any of the required environment variables (KEY, TOKEN, HASH, HOST)
/// are not set. It will also panic if the endpoint URL cannot be parsed.
///
/// # Errors
///
/// Returns a boxed `Error` if any unrecoverable error occurs during HTTP client creation,
/// payload generation, or sending the payload (after retries).
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = env::var("KEY").expect("KEY environment variable not set");
    let token = env::var("TOKEN").expect("TOKEN environment variable not set");
    let hash = env::var("HASH").expect("HASH environment variable not set");
    let host = env::var("HOST").expect("HOST environment variable not set");
    let endpoint_str = format!("https://{}/data/{}/", host, hash);
    let endpoint = Url::parse(&endpoint_str)?; // Propagate parse error

    let sleep_duration_str = env::var("SLEEP_DURATION").unwrap_or_else(|_| "5".to_string());
    let sleep_duration: u64 = sleep_duration_str.parse().unwrap_or(1);
    let sleep_duration = sleep_duration.max(1) * 60;

    let client = Client::builder()
        .user_agent("RustyIP")
        .build()?;

    loop {
        let data = generate_payload(&client, &host, &token, &key).await?;

        let params = [("payload", data.to_string())];
        
        // Use AppClient trait's post_form method via reqwest::Client
        let send_with_retry = || async { client.post_form(endpoint.clone(), &params).await };
        retry(send_with_retry, RETRY_DELAY, 3).await?;

        sleep(Duration::from_secs(sleep_duration)).await;
    }
}
