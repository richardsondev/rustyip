use rand::RngExt;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD as B64URL};
use ed25519_dalek::{Signer, SigningKey};
use reqwest::{Client, Url};
use std::env;
use std::net::Ipv4Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use std::future::Future;

const HEX_CHARS: &[u8] = b"abcdef0123456789";
const RETRY_DELAY: u64 = 30;
/// Domain-separation string bound into every token as `iss`/`aud`.
const TOKEN_AUD: &str = "rustyip-ddns/v2";
/// Token freshness window in seconds; the backend enforces the same bound.
const TOKEN_TTL_SECS: u64 = 120;

/// Optional compiled-in configuration.
///
/// Reserves a fixed, file-backed region — a 16-byte `MAGIC` marker followed by
/// `CONFIG_CAPACITY` payload bytes — that may hold a minified JSON object of
/// configuration values written into the region after build, terminated by a
/// single NUL byte, e.g. `{"HOST":"..","HASH":"..","KEY":".."}`. Values found
/// here take precedence over the corresponding environment variables.
#[cfg(feature = "embed-config")]
mod embedded {
    use std::sync::OnceLock;

    /// Marker delimiting the start of the region. Must stay free of NUL bytes.
    pub const MAGIC: [u8; 16] = *b"<RUSTYIP-CFGv1>\n";
    /// Bytes reserved for the JSON payload (after the marker).
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
    // A compiled-in value, when present, takes precedence; otherwise fall back
    // to the environment variable of the same name.
    #[cfg(feature = "embed-config")]
    {
        if let Some(v) = embedded::config()
            .and_then(|cfg| cfg.get(name))
            .and_then(|val| val.as_str())
        {
            return Some(v.to_string());
        }
    }
    if let Ok(v) = env::var(name) {
        return Some(v);
    }
    None
}

/// Gets a required configuration value, or returns an error if it is set by
/// neither a compiled-in value nor an environment variable.
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

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Parses a base64 (standard or URL-safe) 32-byte Ed25519 private seed.
fn parse_signing_key(key_b64: &str) -> Result<SigningKey, Box<dyn std::error::Error>> {
    let key_b64 = key_b64.trim();
    let raw = B64
        .decode(key_b64)
        .or_else(|_| B64URL.decode(key_b64))
        .map_err(|_| "KEY must be base64-encoded")?;
    let seed: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| "KEY must decode to exactly 32 bytes (an Ed25519 private seed)")?;
    Ok(SigningKey::from_bytes(&seed))
}

fn b64url_json(value: &serde_json::Value) -> String {
    B64URL.encode(value.to_string().as_bytes())
}

/// Builds a compact EdDSA-signed JWT asserting the current WAN IP for `account`.
///
/// Freshness is carried by `iat`/`exp` and replay is deterred by a random
/// `jti`; the backend verifies the signature with the matching public key.
fn build_token(signing_key: &SigningKey, kid: &str, account: &str, wanip: &str) -> String {
    let now = now_unix();
    let header = serde_json::json!({ "alg": "EdDSA", "typ": "JWT", "kid": kid });
    let claims = serde_json::json!({
        "iss": TOKEN_AUD,
        "aud": TOKEN_AUD,
        "sub": account,
        "ip": wanip,
        "iat": now,
        "exp": now + TOKEN_TTL_SECS,
        "jti": random_hex(32),
    });
    let signing_input = format!("{}.{}", b64url_json(&header), b64url_json(&claims));
    let signature = signing_key.sign(signing_input.as_bytes());
    format!("{}.{}", signing_input, B64URL.encode(signature.to_bytes()))
}

/// Generates a fresh Ed25519 keypair and prints it for provisioning: the
/// private seed goes in `KEY` on the client, the public key on the backend.
fn keygen() {
    let mut rng = rand::rng();
    let seed: [u8; 32] = std::array::from_fn(|_| rng.random_range(0u8..=255));
    let signing_key = SigningKey::from_bytes(&seed);
    println!("KEY (private seed, base64) : {}", B64.encode(seed));
    println!("public key (base64)        : {}", B64.encode(signing_key.verifying_key().to_bytes()));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("keygen") {
        keygen();
        return Ok(());
    }

    let signing_key = parse_signing_key(&get_required_config("KEY")?)?;
    let kid = get_config("KID").unwrap_or_else(|| "1".to_string());
    let account = get_required_config("HASH")?;
    let host = get_required_config("HOST")?;
    let endpoint = Url::parse(&format!("https://{}/data/{}/", host, account))?;
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
        // Fetch the current IP and post a freshly signed token. Failures are
        // logged but we still sleep and continue so the daemon keeps running.
        match get_ip(&client, &host).await {
            Ok(wanip) => {
                let token = build_token(&signing_key, &kid, &account, &wanip);
                let send_with_retry = || client.post(endpoint.clone()).bearer_auth(token.as_str()).send();
                if let Err(e) = retry(send_with_retry, RETRY_DELAY, 3).await {
                    eprintln!("Failed to send update after retries: {e}");
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch current IP: {e}");
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
    fn test_parse_signing_key() {
        assert!(parse_signing_key(&B64.encode([0u8; 32])).is_ok());
        assert!(parse_signing_key(&B64URL.encode([9u8; 32])).is_ok());
        assert!(parse_signing_key(&B64.encode([0u8; 16])).is_err()); // wrong length
        assert!(parse_signing_key("not base64!!!").is_err());
    }

    #[test]
    fn test_token_sign_and_verify() {
        use ed25519_dalek::{Signature, Verifier};

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = signing_key.verifying_key();

        let token = build_token(&signing_key, "1", "acct-123", "203.0.113.7");
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "compact JWT has three segments");

        // --- Reference for the backend verifier ---
        // 1. Decode the header and PIN the algorithm (reject anything but EdDSA).
        let header: serde_json::Value =
            serde_json::from_slice(&B64URL.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "EdDSA");
        assert_eq!(header["typ"], "JWT");

        // 2. Verify the signature over `header.payload` using the PUBLIC key only.
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let sig_bytes: [u8; 64] = B64URL.decode(parts[2]).unwrap().try_into().unwrap();
        public_key
            .verify(signing_input.as_bytes(), &Signature::from_bytes(&sig_bytes))
            .expect("signature must verify with the matching public key");

        // 3. Validate the claims (aud/iss, subject, asserted IP, freshness window).
        let claims: serde_json::Value =
            serde_json::from_slice(&B64URL.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims["aud"], TOKEN_AUD);
        assert_eq!(claims["iss"], TOKEN_AUD);
        assert_eq!(claims["sub"], "acct-123");
        assert_eq!(claims["ip"], "203.0.113.7");
        let (iat, exp) = (claims["iat"].as_u64().unwrap(), claims["exp"].as_u64().unwrap());
        assert_eq!(exp - iat, TOKEN_TTL_SECS);

        // A different key must NOT verify.
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key();
        assert!(other
            .verify(signing_input.as_bytes(), &Signature::from_bytes(&sig_bytes))
            .is_err());
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
