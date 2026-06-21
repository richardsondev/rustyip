use rand::RngExt;
use sha2::Sha512;
use reqwest::{Client, Url};
use std::env;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::Value;
use std::future::Future;
use hmac::{Hmac, Mac};

type HmacSha512 = Hmac<Sha512>;

const HEX_CHARS: &[u8] = b"abcdef0123456789";
const RETRY_DELAY: u64 = 30;

#[cfg(feature = "embed-config")]
const EMBEDDED_CONFIG_SIZE: usize = 8192;

#[cfg(feature = "embed-config")]
#[used]
#[unsafe(no_mangle)]
pub static EMBEDDED_CONFIG: [u8; EMBEDDED_CONFIG_SIZE] = [0; EMBEDDED_CONFIG_SIZE];

#[cfg(feature = "embed-config")]
fn load_embedded_config() -> Option<serde_json::Value> {
    let end = EMBEDDED_CONFIG.iter().position(|&b| b == 0).unwrap_or(EMBEDDED_CONFIG.len());
    if end == 0 {
        return None;
    }
    let s = std::str::from_utf8(&EMBEDDED_CONFIG[..end]).ok()?;
    serde_json::from_str(s).ok()
}

fn get_config(name: &str) -> Option<String> {
    if let Ok(v) = env::var(name) {
        return Some(v);
    }
    #[cfg(feature = "embed-config")]
    {
        if let Some(config) = load_embedded_config() {
            if let Some(v) = config.get(name).and_then(|val| val.as_str()) {
                return Some(v.to_string());
            } else {
                eprintln!("No embedded entry found for {}", name);
            }
        } else {
            eprintln!("No embedded entry found for {}", name);
        }
    }
    None
}

fn random_hex(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len).map(|_| HEX_CHARS[rng.random_range(0..HEX_CHARS.len())] as char).collect()
}

async fn retry<F, Fut, T, E>(mut f: F, delay_seconds: u64, tries: usize) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut delay = delay_seconds;
    for _ in 0..tries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(_) => {
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
    let mut mac = HmacSha512::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(wandata_str.as_bytes());
    let result = mac.finalize();
    let hex_string: String = result.into_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    Ok(serde_json::json!({
        "status": "success",
        "data": hex_string,
        "additional": format!("{}{}", salta, saltb)
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = get_config("KEY").expect("KEY is required");
    let token = get_config("TOKEN").expect("TOKEN is required");
    let hash = get_config("HASH").expect("HASH is required");
    let host = get_config("HOST").expect("HOST is required");
    let endpoint = Url::parse(&format!("https://{}/data/{}/", host, hash))?;
    let sleep_str = get_config("SLEEP_DURATION").unwrap_or_else(|| "5".to_string());
    let sleep_duration: u64 = sleep_str.parse().unwrap_or(1);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_from_env() {
        unsafe { std::env::set_var("TEST_KEY", "env_value"); }
        let val = get_config("TEST_KEY");
        assert_eq!(val, Some("env_value".to_string()));
        unsafe { std::env::remove_var("TEST_KEY"); }
    }

    #[test]
    fn test_get_config_missing() {
        unsafe { std::env::remove_var("NONEXISTENT_VAR"); }
        let val = get_config("NONEXISTENT_VAR");
        // When embed-config on, it will log "No embedded..." and return None
        assert_eq!(val, None);
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_load_embedded_config() {
        // With embed-config, buffer is zeros -> None or empty
        let cfg = load_embedded_config();
        assert!(cfg.is_none() || cfg.unwrap().as_object().map_or(true, |o| o.is_empty()));
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_embedded_buffer_size() {
        assert_eq!(EMBEDDED_CONFIG.len(), 8192);
    }
}
