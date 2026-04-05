// Telegraph service for publishing long content to Telegraph pages.
// Ported from Go version: internal/services/telegraph/telegraph.go

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;
use tracing::{info, warn};

/// Telegraph node representation (public for callers to compose pages).
#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeChild>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum NodeChild {
    Text(String),
    Node(Box<Node>),
}

const DEFAULT_API_URL: &str = "https://api.telegra.ph";
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);
const PAGE_CREATE_INTERVAL: Duration = Duration::from_secs(2);
const PAGE_SIZE_LIMIT: usize = 60 * 1024; // 60 KB
const SAFETY_BUFFER: usize = 2 * 1024; // 2 KB

#[derive(Debug, Clone)]
pub struct TelegraphConfig {
    pub access_token: String,
    pub api_url: String,
    pub author_name: String,
    pub timeout_secs: u64,
}

impl TelegraphConfig {
    pub fn from_env() -> Option<Self> {
        let access_token = std::env::var("TELEGRAPH_ACCESS_TOKEN").ok()?;
        if access_token.is_empty() {
            return None;
        }

        let author_name =
            std::env::var("TELEGRAPH_AUTHOR_NAME").unwrap_or_else(|_| "Insights Bot".to_string());

        let api_url =
            std::env::var("TELEGRAPH_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
        let timeout_secs = std::env::var("TELEGRAPH_TIMEOUT_SEC")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        Some(Self {
            access_token,
            api_url,
            author_name,
            timeout_secs,
        })
    }
}

#[derive(Clone)]
pub struct TelegraphService {
    config: TelegraphConfig,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct TelegraphResponse<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegraphPage {
    pub path: String,
    pub url: String,
    pub title: String,
}

impl TelegraphService {
    pub fn new(config: TelegraphConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("failed to build HTTP client");

        Self { config, client }
    }

    pub fn from_env() -> Option<Self> {
        TelegraphConfig::from_env().map(Self::new)
    }

    /// Create a new Telegraph page from pre-built nodes.
    pub async fn create_page_nodes(&self, title: &str, nodes: &[Node]) -> Result<String> {
        let content = serialize_nodes(nodes)?;
        let title_owned = title.to_string();
        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            let url = format!("{}/createPage", self.config.api_url);
            let resp = self
                .client
                .post(&url)
                .form(&[
                    ("access_token", &self.config.access_token),
                    ("title", &title_owned),
                    ("author_name", &self.config.author_name),
                    ("content", &content),
                    ("return_content", &"false".to_string()),
                ])
                .send()
                .await;

            match resp {
                Ok(response) => {
                    let body: TelegraphResponse<TelegraphPage> = response
                        .json()
                        .await
                        .context("failed to parse Telegraph response")?;

                    if body.ok
                        && let Some(page) = body.result
                    {
                        info!(
                            url = %page.url,
                            path = %page.path,
                            title = %title,
                            node_len = content.len(),
                            "created Telegraph page (nodes)"
                        );
                        return Ok(page.url);
                    }

                    let error_msg = body.error.unwrap_or_else(|| "unknown error".to_string());
                    last_error = Some(anyhow::anyhow!("Telegraph API error: {}", error_msg));
                }
                Err(e) => {
                    last_error = Some(e.into());
                }
            }

            if attempt < MAX_RETRIES {
                warn!(
                    attempt = attempt,
                    title = %title,
                    node_len = content.len(),
                    "failed to create Telegraph page (nodes), retrying..."
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("failed to create Telegraph page (nodes)")))
    }

    /// Create single or paginated pages from nodes.
    pub async fn create_page_auto_nodes(&self, title: &str, nodes: &[Node]) -> Result<Vec<String>> {
        let pages = split_nodes(nodes, PAGE_SIZE_LIMIT - SAFETY_BUFFER)?;
        let mut urls = Vec::new();

        for (i, page_nodes) in pages.iter().enumerate() {
            let page_title = if i == 0 {
                title.to_string()
            } else {
                format!("{} (Part {})", title, i + 1)
            };
            match self.create_page_nodes(&page_title, page_nodes).await {
                Ok(url) => urls.push(url),
                Err(e) => {
                    warn!(
                        error = %e,
                        part = i + 1,
                        total = pages.len(),
                        "failed to create page in series (nodes)"
                    );
                }
            }
            if i < pages.len() - 1 {
                tokio::time::sleep(PAGE_CREATE_INTERVAL).await;
            }
        }

        if urls.is_empty() {
            return Err(anyhow::anyhow!(
                "failed to create any pages in series (nodes)"
            ));
        }
        Ok(urls)
    }
}

/// Serialize nodes to JSON.
pub fn serialize_nodes(nodes: &[Node]) -> Result<String> {
    serde_json::to_string(nodes).context("failed to serialize telegraph nodes")
}

/// Split nodes into multiple pages based on byte length.
fn split_nodes(nodes: &[Node], max_len: usize) -> Result<Vec<Vec<Node>>> {
    let mut pages: Vec<Vec<Node>> = Vec::new();
    let mut current: Vec<Node> = Vec::new();
    let mut current_len = 2; // for [] brackets

    for node in nodes {
        let node_json = serde_json::to_string(node)?;
        let node_len = node_json.len() + 1; // comma
        if !current.is_empty() && current_len + node_len > max_len {
            pages.push(current);
            current = Vec::new();
            current_len = 2;
        }
        current.push(node.clone());
        current_len += node_len;
    }

    if !current.is_empty() {
        pages.push(current);
    }

    Ok(pages)
}
