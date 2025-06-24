use std::io::Read;
use std::str::FromStr;

use crate::digest::OciDigest;
use crate::models::{Descriptor, ImageManifest};
use anyhow::Result;
use bytes::Bytes;
use reqwest::{header, Client as ReqwestClient, StatusCode};
use serde::Deserialize;
use serde_json::Value;

/// A client for interacting with an OCI registry.
pub struct Client {
    registry_url: String,
    auth: Option<String>,
    client: ReqwestClient,
}

impl Client {
    /// Create a new client for the given registry URL.
    pub fn new(registry_url: String, auth: Option<String>) -> Self {
        Self {
            registry_url,
            auth,
            client: ReqwestClient::new(),
        }
    }

    /// Create a new session for the given repository.
    pub fn new_session(&self, repository: String) -> ClientSession {
        ClientSession {
            repository,
            registry_url: self.registry_url.clone(),
            client: self.client.clone(),
            auth: self.auth.clone(),
            token: None,
        }
    }

    /// List all repositories in the registry.
    pub async fn list_repositories(&self) -> Result<Vec<String>> {
        let url = format!("{}/v2/_catalog", self.registry_url);

        // Create the request
        let mut request = self.client.get(&url);

        // Add authentication if available
        if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        let response = request.send().await?;

        // Handle authentication if needed
        let final_response = if response.status() == StatusCode::UNAUTHORIZED {
            // Get the WWW-Authenticate header
            if let Some(auth_header) = response.headers().get(header::WWW_AUTHENTICATE) {
                // Parse the WWW-Authenticate header
                let auth_header = auth_header.to_str()?;
                if auth_header.starts_with("Bearer ") {
                    // Extract the realm, service, and scope from the header
                    let mut realm = None;
                    let mut service = None;
                    let mut scope = None;

                    for part in auth_header["Bearer ".len()..].split(',') {
                        let part = part.trim();
                        if let Some(eq_pos) = part.find('=') {
                            let (key, value) = part.split_at(eq_pos);
                            let value = &value[1..]; // Skip the '='
                            let value = value.trim_matches('"');

                            match key {
                                "realm" => realm = Some(value.to_string()),
                                "service" => service = Some(value.to_string()),
                                "scope" => scope = Some(value.to_string()),
                                _ => {}
                            }
                        }
                    }

                    // If we have a realm, try to get a token
                    if let Some(realm) = realm {
                        // Build the token request URL
                        let mut token_url = reqwest::Url::parse(&realm)?;

                        // Add query parameters
                        if let Some(service) = service {
                            token_url.query_pairs_mut().append_pair("service", &service);
                        }
                        if let Some(scope) = scope {
                            token_url.query_pairs_mut().append_pair("scope", &scope);
                        }

                        // Make the token request
                        let mut token_request = self.client.get(token_url);

                        // Add basic auth if we have it
                        if let Some(auth) = &self.auth {
                            token_request = token_request.header(header::AUTHORIZATION, format!("Basic {}", auth));
                        }

                        // Send the token request
                        let token_response = token_request.send().await?;

                        if token_response.status().is_success() {
                            // Parse the token response
                            let token_data: TokenResponse = token_response.json().await?;

                            // Retry the original request with the token
                            let retry_request = self.client.get(&url)
                                .header(header::AUTHORIZATION, format!("Bearer {}", token_data.token));

                            retry_request.send().await?
                        } else {
                            response
                        }
                    } else {
                        response
                    }
                } else {
                    response
                }
            } else {
                response
            }
        } else {
            response
        };

        if final_response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to list repositories: {}", final_response.status()));
        }

        let catalog: Value = final_response.json().await?;
        let repositories = catalog["repositories"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid catalog response"))?
            .iter()
            .filter_map(|repo| repo.as_str().map(|s| s.to_string()))
            .collect();

        Ok(repositories)
    }

    /// Check if the registry API is available.
    pub async fn check_api(&self) -> Result<bool> {
        let url = format!("{}/v2/", self.registry_url);

        // Create the request
        let mut request = self.client.get(&url);

        // Add authentication if available
        if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        let response = request.send().await?;

        // If we get a 401, the API is still available but requires authentication
        if response.status() == StatusCode::UNAUTHORIZED {
            return Ok(true);
        }

        Ok(response.status() == StatusCode::OK)
    }
}

/// A session for interacting with a specific repository in an OCI registry.
pub struct ClientSession {
    repository: String,
    registry_url: String,
    client: ReqwestClient,
    auth: Option<String>,
    token: Option<String>,
}

// Token authentication response from the auth service
#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
    expires_in: Option<u64>,
    issued_at: Option<String>,
}

impl ClientSession {
    /// Handle authentication for requests to the registry.
    /// This method will automatically obtain a token if needed.
    async fn authenticate_request(&mut self, url: &str, method: reqwest::Method) -> Result<reqwest::Response> {
        // Create a new request
        let mut request = self.client.request(method.clone(), url);

        // If we have a token, add it to the request
        if let Some(token) = &self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }
        // If we have basic auth credentials, add them to the request
        else if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        // Try the request
        let response = request.send().await?;

        // If we get a 401 Unauthorized response, try to get a token
        if response.status() == StatusCode::UNAUTHORIZED {
            // Get the WWW-Authenticate header
            if let Some(auth_header) = response.headers().get(header::WWW_AUTHENTICATE) {
                // Parse the WWW-Authenticate header
                let auth_header = auth_header.to_str()?;
                if auth_header.starts_with("Bearer ") {
                    // Extract the realm, service, and scope from the header
                    let mut realm = None;
                    let mut service = None;
                    let mut scope = None;

                    for part in auth_header["Bearer ".len()..].split(',') {
                        let part = part.trim();
                        if let Some(eq_pos) = part.find('=') {
                            let (key, value) = part.split_at(eq_pos);
                            let value = &value[1..]; // Skip the '='
                            let value = value.trim_matches('"');

                            match key {
                                "realm" => realm = Some(value.to_string()),
                                "service" => service = Some(value.to_string()),
                                "scope" => scope = Some(value.to_string()),
                                _ => {}
                            }
                        }
                    }

                    // If we have a realm, try to get a token
                    if let Some(realm) = realm {
                        // Build the token request URL
                        let mut token_url = reqwest::Url::parse(&realm)?;

                        // Add query parameters
                        if let Some(service) = service {
                            token_url.query_pairs_mut().append_pair("service", &service);
                        }
                        if let Some(scope) = scope {
                            token_url.query_pairs_mut().append_pair("scope", &scope);
                        }

                        // Make the token request
                        let mut token_request = self.client.get(token_url);

                        // Add basic auth if we have it
                        if let Some(auth) = &self.auth {
                            token_request = token_request.header(header::AUTHORIZATION, format!("Basic {}", auth));
                        }

                        // Send the token request
                        let token_response = token_request.send().await?;

                        if token_response.status().is_success() {
                            // Parse the token response
                            let token_data: TokenResponse = token_response.json().await?;

                            // Store the token
                            self.token = Some(token_data.token);

                            // Retry the original request with the token
                            let retry_request = self.client.request(method, url)
                                .header(header::AUTHORIZATION, format!("Bearer {}", self.token.as_ref().unwrap()));

                            return Ok(retry_request.send().await?);
                        }
                    }
                }
            }
        }

        // If we didn't need to authenticate or couldn't authenticate, return the original response
        Ok(response)
    }

    /// Make an authenticated GET request to the registry.
    async fn authenticated_get(&mut self, url: &str) -> Result<reqwest::Response> {
        self.authenticate_request(url, reqwest::Method::GET).await
    }

    /// Make an authenticated HEAD request to the registry.
    async fn authenticated_head(&mut self, url: &str) -> Result<reqwest::Response> {
        self.authenticate_request(url, reqwest::Method::HEAD).await
    }

    /// Make an authenticated POST request to the registry.
    async fn authenticated_post(&mut self, url: &str) -> Result<reqwest::Response> {
        self.authenticate_request(url, reqwest::Method::POST).await
    }

    /// Make an authenticated PUT request to the registry with a body.
    async fn authenticated_put(&mut self, url: &str, body: impl Into<reqwest::Body>) -> Result<reqwest::Response> {
        let method = reqwest::Method::PUT;

        // Create a new request with the body
        let mut request = self.client.request(method.clone(), url).body(body);

        // If we have a token, add it to the request
        if let Some(token) = &self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }
        // If we have basic auth credentials, add them to the request
        else if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        // Send the request
        let response = request.send().await?;

        // Handle authentication if needed
        if response.status() == StatusCode::UNAUTHORIZED {
            // Get a new token and retry
            let auth_response = self.authenticate_request(url, method).await?;
            Ok(auth_response)
        } else {
            Ok(response)
        }
    }

    /// Make an authenticated PATCH request to the registry with a body.
    async fn authenticated_patch(&mut self, url: &str, body: impl Into<reqwest::Body>) -> Result<reqwest::Response> {
        let method = reqwest::Method::PATCH;

        // Create a new request with the body
        let mut request = self.client.request(method.clone(), url).body(body);

        // If we have a token, add it to the request
        if let Some(token) = &self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }
        // If we have basic auth credentials, add them to the request
        else if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        // Send the request
        let response = request.send().await?;

        // Handle authentication if needed
        if response.status() == StatusCode::UNAUTHORIZED {
            // Get a new token and retry
            let auth_response = self.authenticate_request(url, method).await?;
            Ok(auth_response)
        } else {
            Ok(response)
        }
    }

    /// Make an authenticated DELETE request to the registry.
    async fn authenticated_delete(&mut self, url: &str) -> Result<reqwest::Response> {
        self.authenticate_request(url, reqwest::Method::DELETE).await
    }
    /// Upload content to the registry.
    pub async fn upload_content<R: Read + Send>(
        &mut self,
        media_type: String,
        mut content: R,
    ) -> Result<Descriptor> {
        // Read the content into a buffer
        let mut buffer = Vec::new();
        content.read_to_end(&mut buffer)?;

        // Upload the buffer
        self.upload_bytes(media_type, &buffer).await
    }

    /// Upload content from a byte slice.
    pub async fn upload_bytes(
        &mut self,
        media_type: String,
        content: &[u8],
    ) -> Result<Descriptor> {
        // Calculate digest
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let digest = format!("sha256:{}", hex::encode(hasher.finalize()));

        // Start upload
        let start_url = format!("{}/v2/{}/blobs/uploads/", self.registry_url, self.repository);
        println!("Starting upload with URL: {}", start_url);
        let start_response = self.authenticated_post(&start_url).await?;

        if start_response.status() != StatusCode::ACCEPTED {
            let status = start_response.status();
            let error_body = start_response.text().await?;
            println!("Error response body: {}", error_body);
            return Err(anyhow::anyhow!("Failed to start upload: {} - {}", status, error_body));
        }

        // Get the upload location
        let location = start_response
            .headers()
            .get("location")
            .ok_or_else(|| anyhow::anyhow!("No location header in response"))?
            .to_str()?;

        let upload_url = if location.starts_with("http") {
            location.to_string()
        } else {
            format!("{}{}", self.registry_url, location)
        };

        // Complete upload
        let complete_url = format!("{}?digest={}", upload_url, digest);
        let complete_response = self.authenticated_put(&complete_url, content.to_vec()).await?;

        if complete_response.status() != StatusCode::CREATED {
            return Err(anyhow::anyhow!("Failed to complete upload: {}", complete_response.status()));
        }

        // Return the descriptor
        Ok(Descriptor {
            media_type,
            digest: OciDigest::from_str(&digest)?,
            size: content.len(),
        })
    }

    /// Check if a blob with the given digest exists.
    pub async fn blob_exists(&mut self, digest: &OciDigest) -> Result<bool> {
        let url = format!("{}/v2/{}/blobs/{}", self.registry_url, self.repository, digest);
        let response = self.authenticated_head(&url).await?;
        Ok(response.status() == StatusCode::OK)
    }

    /// Fetch a blob with the given digest.
    pub async fn fetch_blob(&mut self, digest: &OciDigest) -> Result<Bytes> {
        let url = format!("{}/v2/{}/blobs/{}", self.registry_url, self.repository, digest);
        let response = self.authenticated_get(&url).await?;

        if response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to fetch blob: {}", response.status()));
        }

        Ok(response.bytes().await?)
    }

    /// Register a manifest with the given reference.
    pub async fn register_manifest(
        &mut self,
        reference: &str,
        manifest: &ImageManifest,
    ) -> Result<()> {
        let url = format!("{}/v2/{}/manifests/{}", self.registry_url, self.repository, reference);

        // Create the request with the manifest as JSON
        let mut request = self.client.request(reqwest::Method::PUT, &url)
            .header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
            .json(manifest);

        // Add authentication
        if let Some(token) = &self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        } else if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        let response = request.send().await?;

        // Handle authentication if needed
        if response.status() == StatusCode::UNAUTHORIZED {
            // Get a new token and retry
            let json_body = serde_json::to_vec(manifest)?;
            let auth_response = self.authenticated_put(&url, json_body).await?;

            if auth_response.status() != StatusCode::CREATED && auth_response.status() != StatusCode::OK {
                return Err(anyhow::anyhow!("Failed to register manifest: {}", auth_response.status()));
            }
        } else if response.status() != StatusCode::CREATED && response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to register manifest: {}", response.status()));
        }

        Ok(())
    }

    /// Query a manifest with the given reference.
    pub async fn query_manifest(
        &mut self,
        reference: &str,
    ) -> Result<Option<ImageManifest>> {
        let url = format!("{}/v2/{}/manifests/{}", self.registry_url, self.repository, reference);

        // Create the request with the appropriate Accept header
        let mut request = self.client.request(reqwest::Method::GET, &url)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json");

        // Add authentication
        if let Some(token) = &self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        } else if let Some(auth) = &self.auth {
            request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
        }

        let response = request.send().await?;

        // Handle authentication if needed
        let final_response = if response.status() == StatusCode::UNAUTHORIZED {
            // Get a new token and retry
            self.authenticated_get(&url).await?
        } else {
            response
        };

        if final_response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if final_response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to query manifest: {}", final_response.status()));
        }

        let manifest = final_response.json().await?;
        Ok(Some(manifest))
    }

    /// Upload content in chunks.
    pub async fn upload_chunked(
        &mut self,
        media_type: String,
        content: &[u8],
        chunk_size: usize,
    ) -> Result<Descriptor> {
        // Start the upload
        let start_url = format!("{}/v2/{}/blobs/uploads/", self.registry_url, self.repository);
        let start_response = self.authenticated_post(&start_url).await?;

        if start_response.status() != StatusCode::ACCEPTED {
            return Err(anyhow::anyhow!("Failed to start upload: {}", start_response.status()));
        }

        // Get the upload location
        let location = start_response
            .headers()
            .get("location")
            .ok_or_else(|| anyhow::anyhow!("No location header in response"))?
            .to_str()?;

        let upload_url = if location.starts_with("http") {
            location.to_string()
        } else {
            format!("{}{}", self.registry_url, location)
        };

        // Calculate digest for verification
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let expected_digest = format!("sha256:{}", hex::encode(hasher.finalize()));

        // Upload chunks
        let mut offset = 0;

        while offset < content.len() {
            let end = std::cmp::min(offset + chunk_size, content.len());
            let chunk = &content[offset..end];

            // Upload chunk
            let mut request = self.client.request(reqwest::Method::PATCH, &upload_url)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", chunk.len())
                .header("Range", format!("{}-{}", offset, end - 1))
                .body(chunk.to_vec());

            // Add authentication
            if let Some(token) = &self.token {
                request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
            } else if let Some(auth) = &self.auth {
                request = request.header(header::AUTHORIZATION, format!("Basic {}", auth));
            }

            let chunk_response = request.send().await?;

            // Handle authentication if needed
            if chunk_response.status() == StatusCode::UNAUTHORIZED {
                // Get a new token and retry with authenticated_patch
                let auth_response = self.authenticated_patch(&upload_url, chunk.to_vec()).await?;

                if auth_response.status() != StatusCode::ACCEPTED {
                    return Err(anyhow::anyhow!("Failed to upload chunk: {}", auth_response.status()));
                }
            } else if chunk_response.status() != StatusCode::ACCEPTED {
                return Err(anyhow::anyhow!("Failed to upload chunk: {}", chunk_response.status()));
            }

            offset = end;
        }

        // Complete the upload
        let complete_url = format!("{}?digest={}", upload_url, expected_digest);
        let complete_response = self.authenticated_put(&complete_url, Vec::<u8>::new()).await?;

        if complete_response.status() != StatusCode::CREATED {
            return Err(anyhow::anyhow!("Failed to complete upload: {}", complete_response.status()));
        }

        // Return the descriptor
        Ok(Descriptor {
            media_type,
            digest: OciDigest::from_str(&expected_digest)?,
            size: content.len(),
        })
    }

    /// List all tags for the repository.
    pub async fn list_tags(&mut self) -> Result<Vec<String>> {
        let url = format!(
            "{}/v2/{}/tags/list",
            self.registry_url,
            self.repository
        );

        let response = self.authenticated_get(&url).await?;

        if response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to list tags: {}", response.status()));
        }

        let tags_list: Value = response.json().await?;
        let tags = tags_list["tags"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid tags response"))?
            .iter()
            .filter_map(|tag| tag.as_str().map(|s| s.to_string()))
            .collect();

        Ok(tags)
    }
}
