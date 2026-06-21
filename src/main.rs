use rand::RngExt;
use sha2::{Digest, Sha512};
use reqwest::{Client, Url};
use std::env;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::Value;
use std::future::Future;

const HEX_CHARS: &[u8] = b"abcdef0123456789";
const RETRY_DELAY: u64 = 30;

#[cfg(feature = "embed-config")]
const EMBEDDED_CONFIG_SIZE: usize = 8192;

/// Large zeroed buffer (8 KiB) reserved so that a web portal can inject a JSON
/// configuration blob at download time by overwriting this region in the
/// compiled binary.
///
/// When the `embed-config` feature is enabled, the runtime will try to parse
/// a JSON object from the start of this buffer (null-terminated) as a fallback
/// for missing environment variables.
///
/// The buffer is only present when the feature is enabled. By default the
/// feature is off to avoid including unnecessary space in the binary.
///
/// The portal patcher should locate this region (e.g. via symbol, section, or
/// by searching for a large run of zero bytes near the binary's data sections)
/// and write a minified JSON like:
///   {"KEY":"...","TOKEN":"...","HOST":"...","HASH":"...","SLEEP_DURATION":"..."} 
/// followed by a null byte.
#[cfg(feature = "embed-config")]
#[used]
static EMBEDDED_CONFIG: [u8; EMBEDDED_CONFIG_SIZE] = [0; EMBEDDED_CONFIG_SIZE];

#[cfg(feature = "embed-config")]
fn load_embedded_config() -> Option<serde_json::Value> {
    let end = EMBEDDED_CONFIG.iter().position(|&b| b == 0).unwrap_or(EMBEDDED_CONFIG.len());
    if end == 0 {
        return None;
    }
    let s = std::str::from_utf8(&EMBEDDED_CONFIG[..end]).ok()?;
    serde_json::from_str(s.trim()).ok()
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

/// Gets a required configuration value.
///
/// Tries environment variable first. If the feature is enabled, falls back to
/// the embedded JSON config. If still missing, returns an error.
fn get_required_config(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(v) = get_config(name) {
        return Ok(v);
    }
    Err(format!("{name} is required (set via environment variable or embedded config)").into())
}

fn random_hex(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len).map(|_| HEX_CHARS[rng.random_range(0..HEX_CHARS.len())] as char).collect()
}

async fn retry<F, Fut, T, E>(mut f: F, initial_delay_seconds: u64, max_tries: usize) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut delay = initial_delay_seconds;
    let mut last_err: Option<E> = None;

    for attempt in 0..max_tries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < max_tries {
                    sleep(Duration::from_secs(delay)).await;
                    delay *= 2;
                }
            }
        }
    }

    // Return the last error after exhausting retries
    Err(last_err.expect("retry loop should have at least one error"))
}

async fn get_ip(client: &Client, host: &str) -> Result<String, Box<dyn std::error::Error>> {
    let fetch_ip = || async {
        let url = Url::parse(&format!("https://{}/ip.txt", host))?;
        let response = client.get(url).send().await?;
        if response.status().is_success()
            && let Ok(ip_addr) = response.text().await?.parse::<Ipv4Addr>()
        {
            return Ok(ip_addr.to_string());
        }
        Err("Failed to get a valid IP address".into())
    };
    retry(fetch_ip, RETRY_DELAY, 5).await
}

async fn generate_payload(client: &Client, host: &str, token: &str, key: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let (salta, saltb) = (random_hex(16), random_hex(16));
    let wanip = get_ip(client, host).await?;
    let wandata_str = format!("{}{}{}{}{}", salta, token, wanip, saltb, key);
    let hash = hmac_sha512(key.as_bytes(), wandata_str.as_bytes());
    let hex_string: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

    Ok(serde_json::json!({
        "status": "success",
        "data": hex_string,
        "additional": format!("{}{}", salta, saltb)
    }))
}

fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    let mut k = [0u8; 128];
    if key.len() > 128 {
        let mut hasher = Sha512::new();
        hasher.update(key);
        let h = hasher.finalize();
        k[..64].copy_from_slice(&h);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 128];
    let mut opad = [0x5cu8; 128];
    for i in 0..128 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut hasher = Sha512::new();
    hasher.update(ipad);
    hasher.update(data);
    let inner = hasher.finalize();
    let mut hasher = Sha512::new();
    hasher.update(opad);
    hasher.update(inner);
    hasher.finalize().into()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = get_required_config("KEY")?;
    let token = get_required_config("TOKEN")?;
    let hash = get_required_config("HASH")?;
    let host = get_required_config("HOST")?;
    let endpoint = Url::parse(&format!("https://{}/data/{}/", host, hash))?;
    let sleep_str = get_config("SLEEP_DURATION").unwrap_or_else(|| "5".to_string());
    let sleep_duration: u64 = match sleep_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid SLEEP_DURATION value '{sleep_str}', defaulting to 5 minutes");
            5
        }
    };
    let sleep_duration = sleep_duration.max(1) * 60;

    let client = Client::builder()
        .user_agent("RustyIP")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .https_only(true)
        .build()?;

    loop {
        // Generate payload (includes fetching current IP). Failures are logged
        // but we still sleep and continue so the daemon keeps running.
        match generate_payload(&client, &host, &token, &key).await {
            Ok(data) => {
                let params = [("payload", data.to_string())];
                let send_with_retry = || client.post(endpoint.clone()).form(&params).send();
                if let Err(e) = retry(send_with_retry, RETRY_DELAY, 3).await {
                    eprintln!("Failed to send update after retries: {e}");
                }
            }
            Err(e) => {
                eprintln!("Failed to generate payload: {e}");
            }
        }

        sleep(Duration::from_secs(sleep_duration)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_missing() {
        // Use a name that is extremely unlikely to be set in any environment.
        // This tests the path where neither env var nor embedded config provides a value.
        let val = get_config("NONEXISTENT_VAR_9876543210_VERY_UNIQUE");
        assert_eq!(val, None);
    }

    #[test]
    fn test_get_required_config_error() {
        let err = get_required_config("DEFINITELY_MISSING_1234567890").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("DEFINITELY_MISSING_1234567890"));
        assert!(msg.contains("is required"));
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_load_embedded_config() {
        // With the embed-config feature enabled, the buffer starts as zeros (or
        // will be overwritten by the portal at download time). Expect no valid
        // JSON here in the test binary.
        if let Some(cfg) = load_embedded_config() {
            assert!(cfg.as_object().map_or(true, |o| o.is_empty()));
        }
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_embedded_buffer_size() {
        // Ensures the reserved buffer has the expected capacity when the
        // feature is enabled.
        assert_eq!(EMBEDDED_CONFIG.len(), 8192);
    }
}
