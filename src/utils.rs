use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use std::future::Future;

/// A constant byte slice containing hexadecimal characters (0-9, a-f).
/// Used by `random_hex` for generating random hexadecimal strings.
pub const HEX_CHARS: &[u8] = b"abcdef0123456789";

/// The default initial delay in seconds for the `retry` function.
pub const RETRY_DELAY: u64 = 30;

/// Generates a random hexadecimal string of a specified length.
///
/// # Arguments
///
/// * `len`: The desired length of the hexadecimal string.
///
/// # Returns
///
/// A `String` containing `len` random hexadecimal characters.
///
/// # Examples
///
/// ```rust
/// use crate::utils::random_hex; // Assuming random_hex is in utils.rs
///
/// let hex_string = random_hex(16);
/// assert_eq!(hex_string.len(), 16);
/// assert!(hex_string.chars().all(|c| c.is_ascii_hexdigit()));
/// ```
pub fn random_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| HEX_CHARS[rng.gen_range(0..HEX_CHARS.len())] as char).collect()
}

/// Retries an asynchronous operation a specified number of times with exponential backoff.
///
/// The function `f` is called. If it returns `Ok(value)`, `value` is returned.
/// If it returns `Err(error)`, the function waits for `delay_seconds`, then doubles the delay
/// and retries, up to `tries` times.
///
/// # Arguments
///
/// * `f`: A closure that returns a `Future` which resolves to `Result<T, E>`.
/// * `delay_seconds`: The initial delay in seconds before the first retry.
/// * `tries`: The maximum number of times to try the operation (excluding the initial attempt).
///
/// # Returns
///
/// Returns `Ok(T)` if the operation succeeds within the given tries, otherwise returns the `Err(E)`
/// from the last attempt.
///
/// # Examples
///
/// ```rust
/// use std::io::{Error, ErrorKind};
/// use std::sync::atomic::{AtomicUsize, Ordering};
/// use tokio::time::Duration;
/// use crate::utils::retry; // Assuming retry is in utils.rs and crate is your project name
///
/// #[tokio::main]
/// async fn main() {
///     let counter = AtomicUsize::new(0);
///     let result = retry(
///         || async {
///             if counter.fetch_add(1, Ordering::SeqCst) < 2 {
///                 println!("Attempting operation, will fail...");
///                 Err(Error::new(ErrorKind::Other, "failed attempt"))
///             } else {
///                 println!("Attempting operation, will succeed...");
///                 Ok(42)
///             }
///         },
///         1, // Initial delay of 1 second
///         3  // Try 3 times after the initial attempt
///     ).await;
///
///     assert_eq!(result.unwrap(), 42);
///     assert_eq!(counter.load(Ordering::SeqCst), 3);
/// }
/// ```
pub async fn retry<F, Fut, T, E>(mut f: F, delay_seconds: u64, tries: usize) -> Result<T, E>
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::io::{Error, ErrorKind};

    /// Tests that `random_hex` generates strings of the correct length.
    /// It checks for lengths 0, 10, and 32.
    #[tokio::test]
    async fn test_random_hex_length() {
        assert_eq!(random_hex(0).len(), 0);
        assert_eq!(random_hex(10).len(), 10);
        assert_eq!(random_hex(32).len(), 32);
    }

    /// Tests that `random_hex` generates strings containing only valid hexadecimal characters.
    /// It generates a 100-character string and verifies each character.
    #[tokio::test]
    async fn test_random_hex_chars() {
        let hex_value = random_hex(100);
        assert!(hex_value.chars().all(|c| HEX_CHARS.contains(&(c as u8))), "random_hex produced non-hex characters");
    }

    /// Tests that `retry` returns successfully on the first attempt
    /// when the operation succeeds immediately.
    #[tokio::test]
    async fn test_retry_success_on_first_try() {
        let result = retry(|| async { Ok::<_, Error>(42) }, 1, 3).await;
        assert_eq!(result.unwrap(), 42);
    }

    /// Tests that `retry` returns successfully after a few failed attempts
    /// if the operation eventually succeeds.
    #[tokio::test]
    async fn test_retry_success_on_later_try() {
        let counter = AtomicUsize::new(0);
        let result = retry(
            || async {
                if counter.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err(Error::new(ErrorKind::Other, "failed"))
                } else {
                    Ok(42)
                }
            },
            1,
            3,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    /// Tests that `retry` returns an error after all attempts are exhausted
    /// if the operation consistently fails.
    /// It also verifies that the operation was attempted the correct number of times.
    #[tokio::test]
    async fn test_retry_fails_after_all_tries() {
        let counter = AtomicUsize::new(0);
        let result = retry(
            || async {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(Error::new(ErrorKind::Other, "failed"))
            },
            1,
            3,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 4); // 3 retries + 1 final call
    }
}
