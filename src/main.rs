use hex;
use std::env;
use rand::Rng;
use sha2::{Sha512, Digest};
use reqwest;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use std::net::Ipv4Addr;
use reqwest::Error;
use std::collections::HashMap;

fn random_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdef0123456789".chars().collect();
    (0..len).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

async fn get_ip(client: &reqwest::Client, host: &String) -> Result<String, Box<dyn std::error::Error>> {
    for i in 0..5 {
        let response = client.get(format!("https://{}/ip.txt", host)).send().await?;

        if response.status().is_success() {
            let body = response.text().await?;
            if let Ok(ip_addr) = body.parse::<Ipv4Addr>() {
                return Ok(ip_addr.to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(2u64.pow(i))).await;
    }

    Err("Failed to get a valid IP address after 5 attempts".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("KEY").expect("KEY environment variable not set");
    let token = env::var("TOKEN").expect("TOKEN environment variable not set");
    let hash = env::var("HASH").expect("HASH environment variable not set");
    let host = env::var("HOST").expect("HOST environment variable not set");
    let endpoint = format!("https://{}/data/{}/", host, hash);
    let sleep_duration: u64 = env::var("SLEEP_DURATION").unwrap_or("5".to_string()).parse().unwrap();
    let sleep_duration = std::cmp::max(sleep_duration, 1) * 60;  // ensure minimum 1 minute, convert to seconds

    loop {
        let client = reqwest::Client::builder()
            .user_agent("RustyIP")
            .build()?;

        let salta = random_hex(16);
        let saltb = random_hex(16);
        let wanip = get_ip(&client, &host).await?;
        let salt = format!("{}{}", salta, saltb);
        let wandata_str = format!("{}{}{}{}{}", salta, token, wanip, saltb, key);
        let wandata = Sha512::digest(wandata_str.as_bytes());
        let hex_string = format!("{:x}", wandata);

        let data = json!({
            "status": "success",
            "data": hex_string,
            "additional": salt
        });

        let mut params = HashMap::new();
        params.insert("payload", data.to_string());

        let mut delay = 2;
        for _ in 0..3 {
            let res = client.post(&endpoint).form(&params).send().await;  
            if res.is_ok() && res.unwrap().status().is_success() {
                break;
            } else {
                sleep(Duration::from_secs(delay)).await;
                delay *= 2;  // exponential backoff
            }
        }

        sleep(Duration::from_secs(sleep_duration)).await;
    }
}
