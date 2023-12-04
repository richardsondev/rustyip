use rand::Rng;
use sha2::{Sha512, Digest};
use reqwest::{Client, Url};
use std::env;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::Value;
use std::future::Future;

const HEX_CHARS: &[u8] = b"abcdef0123456789";
const RETRY_DELAY: u64 = 2;
const CONNECTION_RETRY_DELAY: u64 = 180;

fn random_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| HEX_CHARS[rng.gen_range(0..HEX_CHARS.len())] as char).collect()
}

async fn retry<F, Fut, T, E>(mut f: F, delay_seconds: u64, tries: usize) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>
{
    let mut delay = delay_seconds;
    for _ in 0..tries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if let Some(reqwest_error) = e.downcast_ref::<ReqwestError>() {
                    if reqwest_error.is_connect() {
                        // Connection error specific handling
                        eprintln!("Connection error encountered: {:?}", reqwest_error);
                        sleep(Duration::from_secs(EXTENDED_RETRY_DELAY)).await;
                        continue; // Skip the standard delay increase
                    }
                }
                // For other errors, use the standard delay logic
                eprintln!("Error encountered: {:?}", e);
                sleep(Duration::from_secs(delay)).await;
                delay *= 2;
            }
        }
    }
    f().await
}

async fn get_ip(client: &Client, host: &str) -> Result<String, Box<dyn std::error::Error>> {
    let fetch_ip = || async {
        let url = Url::parse(&format!("https://{}/ip.txt", host))?;
        let response = client.get(url).send().await?;
        if response.status().is_success() {
            if let Ok(ip_addr) = response.text().await?.parse::<Ipv4Addr>() {
                return Ok(ip_addr.to_string());
            }
        }
        Err("Failed to get a valid IP address".into())
    };
    retry(fetch_ip, RETRY_DELAY, 5).await
}

async fn generate_payload(client: &Client, host: &str, token: &str, key: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let (salta, saltb) = (random_hex(16), random_hex(16));
    let wanip = get_ip(&client, host).await?;
    let wandata_str = format!("{}{}{}{}{}", salta, token, wanip, saltb, key);
    let hex_string = format!("{:x}", Sha512::digest(wandata_str.as_bytes()));

    Ok(serde_json::json!({
        "status": "success",
        "data": hex_string,
        "additional": format!("{}{}", salta, saltb)
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("KEY").expect("KEY environment variable not set");
    let token = env::var("TOKEN").expect("TOKEN environment variable not set");
    let hash = env::var("HASH").expect("HASH environment variable not set");
    let host = env::var("HOST").expect("HOST environment variable not set");
    let endpoint = Url::parse(&format!("https://{}/data/{}/", host, hash))?;
    let sleep_duration: u64 = env::var("SLEEP_DURATION").unwrap_or_else(|_| "5".to_string()).parse().unwrap_or(1);
    let sleep_duration = sleep_duration.max(1) * 60;

    let client = Client::builder()
        .user_agent("RustyIP")
        .build()?;

    loop {
        let data = generate_payload(&client, &host, &token, &key).await?;

        let params = [("payload", data.to_string())];

        let send_with_retry = || client.post(endpoint.clone()).form(&params).send();
        retry(send_with_retry, RETRY_DELAY, 3).await?;

        sleep(Duration::from_secs(sleep_duration)).await;
    }
}
