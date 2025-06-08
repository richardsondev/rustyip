use reqwest::{Client, Url, Response};
use async_trait::async_trait;

/// A trait defining the asynchronous HTTP client operations required by the application.
///
/// This abstraction allows for mocking the HTTP client in tests.
#[async_trait]
pub trait AppClient {
    /// Performs an asynchronous HTTP GET request to the specified URL.
    ///
    /// # Arguments
    ///
    /// * `url`: The `Url` to send the GET request to.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `reqwest::Response` if successful, or a `reqwest::Error` otherwise.
    async fn get(&self, url: Url) -> Result<Response, reqwest::Error>;

    /// Performs an asynchronous HTTP POST request with form data to the specified URL.
    ///
    /// # Arguments
    ///
    /// * `url`: The `Url` to send the POST request to.
    /// * `params`: A slice of key-value pairs representing the form data.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `reqwest::Response` if successful, or a `reqwest::Error` otherwise.
    async fn post_form(&self, url: Url, params: &[(&str, String)]) -> Result<Response, reqwest::Error>;
}

/// Implementation of `AppClient` for the `reqwest::Client`.
///
/// This allows the standard `reqwest::Client` to be used wherever an `AppClient` is needed.
#[async_trait]
impl AppClient for Client {
    async fn get(&self, url: Url) -> Result<Response, reqwest::Error> {
        self.get(url).send().await
    }

    async fn post_form(&self, url: Url, params: &[(&str, String)]) -> Result<Response, reqwest::Error> {
        self.post(url).form(params).send().await
    }
}
