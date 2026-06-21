use rand::RngExt;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha512;
use reqwest::{Client, Url};
use std::env;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::Value;
use std::future::Future;

type HmacSha512 = Hmac<Sha512>;

const HEX_CHARS: &[u8] = b"abcdef0123456789";
const RETRY_DELAY: u64 = 30;

/// Embedded configuration support.
///
/// When the `embed-config` feature is enabled, the binary reserves a region
/// that a download-time "injector" service can patch with a minified JSON
/// object, providing a fallback for any configuration value not supplied via
/// environment variables.
///
/// Layout: a fixed 16-byte `MAGIC` marker followed by `CONFIG_CAPACITY` bytes of
/// payload space. The injector locates the marker in the on-disk binary and
/// writes minified JSON immediately after it, terminated by a single NUL byte,
/// e.g. `{"KEY":"..","TOKEN":"..","HOST":"..","HASH":"..","SLEEP_DURATION":".."}`.
///
/// Security note: any values injected here (KEY/TOKEN/HASH) are stored in the
/// binary in plaintext and can be recovered by anyone who obtains it. Treat a
/// patched binary as secret material.
#[cfg(feature = "embed-config")]
mod embedded {
    use std::sync::OnceLock;

    /// Marker the injector searches for. Must stay free of NUL bytes.
    pub const MAGIC: [u8; 16] = *b"<RUSTYIP-CFGv1>\n";
    /// Bytes reserved for the injected JSON payload (after the marker).
    pub const CONFIG_CAPACITY: usize = 8192;
    const SIZE: usize = MAGIC.len() + CONFIG_CAPACITY;

    /// Builds a fully non-zero initializer.
    ///
    /// An all-zero `static` would be placed in `.bss` (or the un-stored zero
    /// tail of `.data`) and would therefore NOT exist in the on-disk binary for
    /// the injector to overwrite. Initializing every byte to a non-zero value
    /// forces the whole region into a file-backed data section.
    const fn init() -> [u8; SIZE] {
        let mut buf = [0xFFu8; SIZE];
        let mut i = 0;
        while i < MAGIC.len() {
            buf[i] = MAGIC[i];
            i += 1;
        }
        buf
    }

    /// Reserved, file-backed region. `#[used]` keeps the symbol even if the
    /// optimizer would otherwise consider it dead.
    #[used]
    static BUFFER: [u8; SIZE] = init();

    /// Reads the raw payload injected after the marker, if present.
    ///
    /// Bytes are read with `std::ptr::read_volatile` so the optimizer cannot
    /// constant-fold the (post-compilation patched) contents of this otherwise
    /// immutable static.
    fn read_raw() -> Option<String> {
        let base = BUFFER.as_ptr();
        let mut i = 0;
        while i < MAGIC.len() {
            // SAFETY: `i < SIZE`; the volatile read defeats constant-folding of
            // the externally patched static.
            let b = unsafe { std::ptr::read_volatile(base.add(i)) };
            if b != MAGIC[i] {
                return None;
            }
            i += 1;
        }
        let mut payload = Vec::new();
        let mut j = MAGIC.len();
        while j < SIZE {
            // SAFETY: `j < SIZE`.
            let b = unsafe { std::ptr::read_volatile(base.add(j)) };
            if b == 0 {
                break;
            }
            payload.push(b);
            j += 1;
        }
        if payload.is_empty() {
            return None;
        }
        String::from_utf8(payload).ok()
    }

    /// Returns the parsed embedded config, parsing at most once.
    pub fn config() -> Option<&'static serde_json::Value> {
        static CACHE: OnceLock<Option<serde_json::Value>> = OnceLock::new();
        CACHE
            .get_or_init(|| read_raw().and_then(|s| serde_json::from_str(s.trim()).ok()))
            .as_ref()
    }
}

fn get_config(name: &str) -> Option<String> {
    if let Ok(v) = env::var(name) {
        return Some(v);
    }
    #[cfg(feature = "embed-config")]
    {
        if let Some(v) = embedded::config()
            .and_then(|cfg| cfg.get(name))
            .and_then(|val| val.as_str())
        {
            return Some(v.to_string());
        }
    }
    None
}

/// Gets a required configuration value.
///
/// Tries the environment variable first. If the `embed-config` feature is
/// enabled, falls back to the embedded JSON config. Returns an error if neither
/// provides a value.
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
    // Always attempt at least once so the final `expect` below is unreachable.
    let max_tries = max_tries.max(1);
    const MAX_DELAY_SECONDS: u64 = 600;
    let mut delay = initial_delay_seconds;
    let mut last_err: Option<E> = None;

    for attempt in 0..max_tries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < max_tries {
                    sleep(Duration::from_secs(delay)).await;
                    delay = delay.saturating_mul(2).min(MAX_DELAY_SECONDS);
                }
            }
        }
    }

    // Return the last error after exhausting retries.
    Err(last_err.expect("retry runs at least once, so an error is always present"))
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
    // NOTE: this preimage and the use of `key` as the HMAC key must match the
    // server's verification exactly. Do not change without coordinating the
    // server side.
    let hash = hmac_sha512(key.as_bytes(), wandata_str.as_bytes());
    let hex_string: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

    Ok(serde_json::json!({
        "status": "success",
        "data": hex_string,
        "additional": format!("{}{}", salta, saltb)
    }))
}

fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    // HMAC accepts a key of any length, so `new_from_slice` never errors here.
    let mut mac = HmacSha512::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().into()
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
    fn test_hmac_sha512_rfc4231_vector() {
        // RFC 4231 test case 2 for HMAC-SHA-512 (key "Jefe").
        let mac = hmac_sha512(b"Jefe", b"what do ya want for nothing?");
        let hex: String = mac.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(
            hex,
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_embedded_config_unpatched_is_none() {
        // The unpatched buffer holds the marker plus non-zero filler (no JSON),
        // so it must parse to None in the test binary.
        assert!(embedded::config().is_none());
    }

    #[test]
    #[cfg(feature = "embed-config")]
    fn test_embedded_capacity() {
        // Ensures the reserved payload capacity matches the documented size.
        assert_eq!(embedded::CONFIG_CAPACITY, 8192);
    }
}
