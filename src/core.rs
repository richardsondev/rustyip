use crate::client::AppClient;
use crate::utils::{random_hex, retry, RETRY_DELAY};
use std::net::Ipv4Addr;
use serde_json::Value;
use sha2::{Sha512, Digest};
use reqwest::Url;
use std::error::Error;

/// Fetches the public IP address from a given host.
///
/// This function attempts to retrieve the IP address by making a GET request to `https://{host}/ip.txt`.
/// It uses a retry mechanism with exponential backoff if the request fails.
///
/// # Arguments
///
/// * `client`: A reference to an `AppClient` implementation used to make HTTP requests.
/// * `host`: The hostname from which to fetch the IP address.
///
/// # Returns
///
/// A `Result` containing the IP address as a `String` if successful, or a boxed `Error` otherwise.
pub async fn get_ip<C: AppClient + ?Sized + Send + Sync>(client: &C, host: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let fetch_ip = || async {
        let url_string = format!("https://{}/ip.txt", host);
        let url = Url::parse(&url_string).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        
        let response = client.get(url).await.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        
        if response.status().is_success() {
            let ip_text = response.text().await.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            if let Ok(ip_addr) = ip_text.parse::<Ipv4Addr>() {
                return Ok(ip_addr.to_string());
            }
        }
        Err(Box::<dyn Error + Send + Sync>::from("Failed to get a valid IP address"))
    };
    retry(fetch_ip, RETRY_DELAY, 5).await
}

/// Generates a JSON payload containing a hashed value based on the public IP, token, and key.
///
/// This function first retrieves the public IP address using `get_ip`.
/// It then constructs a string by concatenating a random salt, the token, the IP address,
/// another random salt, and the key. This string is then hashed using SHA512.
/// The resulting payload includes the status, the hex-encoded hash, and the concatenated salts.
///
/// # Arguments
///
/// * `client`: A reference to an `AppClient` implementation used for `get_ip`.
/// * `host`: The hostname used by `get_ip` to fetch the IP address.
/// * `token`: A token string to be included in the hash.
/// * `key`: A key string to be included in the hash.
///
/// # Returns
///
/// A `Result` containing the `serde_json::Value` payload if successful, or a boxed `Error` otherwise.
pub async fn generate_payload<C: AppClient + ?Sized + Send + Sync>(client: &C, host: &str, token: &str, key: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let (salta, saltb) = (random_hex(16), random_hex(16));
    let wanip = get_ip(client, host).await?;
    let wandata_str = format!("{}{}{}{}{}", salta, token, wanip, saltb, key);
    let hex_string = format!("{:x}", Sha512::digest(wandata_str.as_bytes()));

    Ok(serde_json::json!({
        "status": "success",
        "data": hex_string,
        "additional": format!("{}{}", salta, saltb)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::AppClient;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use reqwest::{Response as ReqwestResponse, StatusCode, Url};

    #[derive(Clone)]
    enum MockResponse {
        Success(StatusCode, Option<String>),
        Error(String),
    }

    struct SimpleMockAppClient {
        response_to_give: Mutex<Option<MockResponse>>,
    }

    impl SimpleMockAppClient {
        fn new() -> Self {
            SimpleMockAppClient { response_to_give: Mutex::new(None) }
        }
        fn set_next_response(&self, status: StatusCode, body: Option<String>) {
            *self.response_to_give.lock().unwrap() = Some(MockResponse::Success(status, body));
        }
        fn set_next_error(&self, error_message: &str) {
            *self.response_to_give.lock().unwrap() = Some(MockResponse::Error(error_message.to_string()));
        }
    }

    #[async_trait]
    impl AppClient for SimpleMockAppClient {
        async fn get(&self, _url: Url) -> Result<ReqwestResponse, reqwest::Error> {
            let mock_resp_opt = self.response_to_give.lock().unwrap().take();
            match mock_resp_opt {
                Some(MockResponse::Success(status, body_opt)) => {
                    let body_content = body_opt.unwrap_or_default();
                    
                    let http_response = http::Response::builder()
                        .status(status.as_u16())
                        .body(reqwest::Body::from(body_content))
                        .expect("Failed to build mock HTTP response");
                    
                    let reqwest_response = ReqwestResponse::from(http_response);
                    
                    if !status.is_success() {
                         return Err(reqwest::Error::builder()
                            .url(Url::parse("http://mock-status-error.local").unwrap())
                            .status(status)
                            .build()
                            .unwrap_or_else(|e| panic!("Failed to build error: {}",e))
                        );
                    }
                    Ok(reqwest_response)
                }
                Some(MockResponse::Error(_msg)) => {
                    Err(reqwest::Error::builder()
                        .url(Url::parse("http://mock-error.local").unwrap())
                        .kind(reqwest::error::ErrorKind::Request)
                        .build()
                        .unwrap_or_else(|e| panic!("Failed to build error: {}",e)))
                }
                None => {
                     Err(reqwest::Error::builder()
                        .url(Url::parse("http://default-mock-error.local").unwrap())
                        .kind(reqwest::error::ErrorKind::Request)
                        .build()
                        .unwrap_or_else(|e| panic!("Failed to build error: {}",e)))
                }
            }
        }

        async fn post_form(&self, _url: Url, _params: &[(&str, String)]) -> Result<ReqwestResponse, reqwest::Error> {
            unimplemented!("post_form is not mocked in this set of tests")
        }
    }

    /// Tests the `get_ip` function for a successful IP retrieval.
    /// It mocks a successful HTTP response with a valid IP address.
    #[tokio::test]
    async fn test_get_ip_success() {
        let client = SimpleMockAppClient::new();
        client.set_next_response(StatusCode::OK, Some("1.2.3.4".to_string()));
        let result = get_ip(&client, "mockhost.local").await;
        assert!(result.is_ok(), "Expected OK, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), "1.2.3.4");
    }

    /// Tests the `get_ip` function when the underlying HTTP client call fails.
    /// It mocks a network error from the `AppClient`.
    #[tokio::test]
    async fn test_get_ip_http_failure() {
        let client = SimpleMockAppClient::new();
        client.set_next_error("Simulated network error");
        let result = get_ip(&client, "mockhost.local").await;
        assert!(result.is_err(), "Expected Err, got OK: {:?}", result.ok());
    }

    /// Tests the `get_ip` function when the HTTP response status is not successful (e.g., 404 Not Found).
    /// It mocks a non-2xx HTTP status code.
    #[tokio::test]
    async fn test_get_ip_not_success_status() {
        let client = SimpleMockAppClient::new();
        client.set_next_response(StatusCode::NOT_FOUND, None);
        let result = get_ip(&client, "mockhost.local").await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "Failed to get a valid IP address");
    }

    /// Tests the `get_ip` function when the HTTP response body contains an invalid IP address format.
    /// It mocks a successful HTTP response but with a malformed IP string.
    #[tokio::test]
    async fn test_get_ip_invalid_ip_format() {
        let client = SimpleMockAppClient::new();
        client.set_next_response(StatusCode::OK, Some("not-an-ip".to_string()));
        let result = get_ip(&client, "mockhost.local").await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "Failed to get a valid IP address");
    }

    /// Tests the `generate_payload` function for a successful payload generation.
    /// It mocks a successful IP retrieval and verifies the structure of the generated JSON payload.
    #[tokio::test]
    async fn test_generate_payload_success() {
        let client = SimpleMockAppClient::new();
        client.set_next_response(StatusCode::OK, Some("1.2.3.4".to_string()));
        let result = generate_payload(&client, "mockhost.local", "test_token", "test_key").await;
        assert!(result.is_ok(), "generate_payload failed: {:?}", result.err());
        let payload = result.unwrap();
        assert_eq!(payload["status"], "success");
        assert!(payload["data"].as_str().is_some());
        assert!(payload["additional"].as_str().is_some());
        assert_eq!(payload["additional"].as_str().unwrap().len(), 32);
    }

    /// Tests the `generate_payload` function when the internal call to `get_ip` fails.
    /// It mocks a failure in the `get_ip` function and ensures `generate_payload` propagates the error.
    #[tokio::test]
    async fn test_generate_payload_get_ip_fails() {
        let client = SimpleMockAppClient::new();
        client.set_next_error("Simulated error for get_ip");
        let result = generate_payload(&client, "mockhost.local", "test_token", "test_key").await;
        assert!(result.is_err());
    }
}
