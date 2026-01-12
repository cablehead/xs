use crate::{
    http_client::{HeaderMap, HttpClient},
    Result,
};
use serde::de::DeserializeOwned;
use std::time::Duration;

pub struct UndefinedHttpClient;

impl HttpClient for UndefinedHttpClient {
    fn get<T: DeserializeOwned>(_url: &str, _timeout: Duration, _headers: HeaderMap) -> Result<T> {
        panic!("choose HTTP client (ureq or reqwest) or implement your own");
    }
}
