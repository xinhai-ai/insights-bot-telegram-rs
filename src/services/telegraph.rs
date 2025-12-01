// Telegraph service for publishing long content to Telegraph pages.
// Ported from Go version: internal/services/telegraph/telegraph.go

use anyhow::{Context, Result};
use ego_tree::NodeId;
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

#[derive(Debug, Serialize)]
struct CreatePageRequest {
    access_token: String,
    title: String,
    author_name: String,
    content: String, // JSON array of nodes
    return_content: bool,
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

    /// Create a new Telegraph page with the given title and HTML content.
    pub async fn create_page(&self, title: &str, html: &str) -> Result<String> {
        let content = html_to_telegraph_nodes(html)?;
        let node_len = content.len();

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            let req = CreatePageRequest {
                access_token: self.config.access_token.clone(),
                title: title.to_string(),
                author_name: self.config.author_name.clone(),
                content: content.clone(),
                return_content: false,
            };

            let url = format!("{}/createPage", self.config.api_url);
            let resp = self
                .client
                .post(&url)
                .form(&[
                    ("access_token", &req.access_token),
                    ("title", &req.title),
                    ("author_name", &req.author_name),
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

                    if body.ok {
                        if let Some(page) = body.result {
                            info!(
                                url = %page.url,
                                path = %page.path,
                                title = %title,
                                node_len = node_len,
                                "created Telegraph page"
                            );
                            return Ok(page.url);
                        }
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
                    node_len = node_len,
                    "failed to create Telegraph page, retrying..."
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("failed to create Telegraph page")))
    }

    /// Edit an existing Telegraph page.
    pub async fn edit_page(&self, path: &str, title: &str, html: &str) -> Result<String> {
        let content = html_to_telegraph_nodes(html)?;
        let node_len = content.len();
        let path = path.trim_start_matches("https://telegra.ph/");

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            let url = format!("{}/editPage/{}", self.config.api_url, path);
            let resp = self
                .client
                .post(&url)
                .form(&[
                    ("access_token", &self.config.access_token),
                    ("title", &title.to_string()),
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

                    if body.ok {
                        if let Some(page) = body.result {
                            info!(
                                url = %page.url,
                                path = %path,
                                title = %title,
                                node_len = node_len,
                                "edited Telegraph page"
                            );
                            return Ok(page.url);
                        }
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
                    path = %path,
                    node_len = node_len,
                    "failed to edit Telegraph page, retrying..."
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("failed to edit Telegraph page")))
    }

    /// Create a single page or series depending on content size.
    pub async fn create_page_auto(&self, title: &str, html: &str) -> Result<Vec<String>> {
        if self.needs_paging(html) {
            self.create_page_series(title, html).await
        } else {
            let url = self.create_page(title, html).await?;
            Ok(vec![url])
        }
    }

    /// Create a new Telegraph page from pre-built nodes.
    pub async fn create_page_nodes(&self, title: &str, nodes: &[Node]) -> Result<String> {
        let content = serialize_nodes(nodes)?;
        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            let req = CreatePageRequest {
                access_token: self.config.access_token.clone(),
                title: title.to_string(),
                author_name: self.config.author_name.clone(),
                content: content.clone(),
                return_content: false,
            };

            let url = format!("{}/createPage", self.config.api_url);
            let resp = self
                .client
                .post(&url)
                .form(&[
                    ("access_token", &req.access_token),
                    ("title", &req.title),
                    ("author_name", &req.author_name),
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

                    if body.ok {
                        if let Some(page) = body.result {
                            info!(
                                url = %page.url,
                                path = %page.path,
                                title = %title,
                                node_len = content.len(),
                                "created Telegraph page (nodes)"
                            );
                            return Ok(page.url);
                        }
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

/// Convert cleaned HTML into Telegraph node JSON string.
fn html_to_telegraph_nodes(html: &str) -> Result<String> {
    let cleaned = sanitize_html(html);
    let fragment = scraper::Html::parse_fragment(&cleaned);
    let mut nodes = Vec::new();

    for child in fragment.root_element().children() {
        let child_ref = fragment.tree.get(child.id()).unwrap();
        if let scraper::node::Node::Element(_) = child_ref.value() {
            if let Some(n) = dom_to_node(&fragment, child.id()) {
                nodes.push(n);
            }
        } else if let scraper::node::Node::Text(t) = child_ref.value() {
            let txt = t.text.trim();
            if !txt.is_empty() {
                nodes.push(Node {
                    tag: "p".into(),
                    attrs: None,
                    children: vec![NodeChild::Text(txt.to_string())],
                });
            }
        }
    }

    if nodes.is_empty() {
        let text = fragment.root_element().text().collect::<String>();
        if !text.trim().is_empty() {
            nodes.push(Node {
                tag: "p".into(),
                attrs: None,
                children: vec![NodeChild::Text(text.trim().to_string())],
            });
        }
    }

    serde_json::to_string(&nodes).context("failed to serialize telegraph nodes")
}

fn sanitize_html(html: &str) -> String {
    let allowed_tags: std::collections::HashSet<&str> = [
        "p",
        "br",
        "strong",
        "b",
        "em",
        "i",
        "u",
        "code",
        "pre",
        "ul",
        "ol",
        "li",
        "blockquote",
        "h3",
        "h4",
        "a",
    ]
    .into_iter()
    .collect();
    let allowed_attrs: std::collections::HashSet<&str> = ["href", "title"].into_iter().collect();

    let mut builder = ammonia::Builder::default();
    builder.tags(allowed_tags);
    builder.generic_attributes(allowed_attrs);
    builder.add_tag_attributes("a", &["href", "title"]);
    builder.clean(html).to_string()
}

fn dom_to_node(doc: &scraper::Html, id: NodeId) -> Option<Node> {
    use scraper::node::Node::*;
    let node = doc.tree.get(id)?;
    match node.value() {
        Text(_) => None, // handled by parent
        Element(elem) => {
            let tag = elem.name();
            let allowed = [
                "p",
                "br",
                "strong",
                "b",
                "em",
                "i",
                "u",
                "code",
                "pre",
                "ul",
                "ol",
                "li",
                "blockquote",
                "h3",
                "h4",
                "a",
            ];
            if !allowed.contains(&tag) {
                return None;
            }

            let mut attrs = std::collections::HashMap::new();
            if tag == "a" {
                if let Some(href) = elem.attr("href") {
                    attrs.insert("href".to_string(), href.to_string());
                }
                if let Some(title) = elem.attr("title") {
                    attrs.insert("title".to_string(), title.to_string());
                }
            }

            let mut children = Vec::new();
            for c in node.children() {
                let child_ref = doc.tree.get(c.id())?;
                match child_ref.value() {
                    Text(t) => {
                        let txt = t.text.trim();
                        if !txt.is_empty() {
                            children.push(NodeChild::Text(txt.to_string()));
                        }
                    }
                    _ => {
                        if let Some(child_node) = dom_to_node(doc, c.id()) {
                            children.push(NodeChild::Node(Box::new(child_node)));
                        }
                    }
                }
            }

            if tag == "br" {
                children.push(NodeChild::Text("\n".into()));
            }

            Some(Node {
                tag: tag.to_string(),
                attrs: if attrs.is_empty() { None } else { Some(attrs) },
                children,
            })
        }
        _ => None,
    }
}

/// Format series links HTML.
fn format_series_links(urls: &[String]) -> String {
    let mut links = String::from("<p><strong>Series Pages:</strong></p><ul>");
    for (i, url) in urls.iter().enumerate() {
        links.push_str(&format!("<li><a href=\"{}\">Part {}</a></li>", url, i + 1));
    }
    links.push_str("</ul><hr>");
    links
}

/// Estimate size of content after Telegraph JSON serialization.
fn estimate_content_size(html: &str) -> usize {
    // Use node JSON length for closer approximation.
    html_to_telegraph_nodes(html)
        .map(|s| s.len())
        .unwrap_or(html.len())
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

impl TelegraphService {
    /// Check if content likely exceeds Telegraph limit.
    fn needs_paging(&self, html: &str) -> bool {
        estimate_content_size(html) > (PAGE_SIZE_LIMIT - SAFETY_BUFFER)
    }

    /// Split content into multiple parts if too large.
    pub fn split_content(&self, html: &str, title: &str) -> Vec<String> {
        let content = sanitize_html(html);
        if !self.needs_paging(&content) {
            return vec![content];
        }

        let mut parts = Vec::new();
        let paragraphs: Vec<&str> = content.split("</p>").collect();

        let header_html = "<p><strong>Note:</strong> Due to content length, this has been split into multiple pages.</p><hr>";
        let mut current_part = header_html.to_string();

        for (i, p) in paragraphs.iter().enumerate() {
            let paragraph = if i < paragraphs.len() - 1 || !p.trim().is_empty() {
                format!("{}</p>", p)
            } else {
                String::new()
            };

            if paragraph.is_empty() {
                continue;
            }

            let test_html = format!("{}{}", current_part, paragraph);
            if estimate_content_size(&test_html) >= (PAGE_SIZE_LIMIT - SAFETY_BUFFER) {
                // Save current part and start new one.
                let footer = "<hr><p><em>(This page contains split content. See series pages for full summary.)</em></p>";
                current_part.push_str(footer);
                parts.push(current_part);

                current_part = format!(
                    "<p><strong>{} (Part {})</strong></p><p><strong>Note:</strong> This is a continuation page.</p><hr>{}",
                    title,
                    parts.len() + 1,
                    paragraph
                );
            } else {
                current_part.push_str(&paragraph);
            }
        }

        if !current_part.is_empty() && current_part != header_html {
            current_part.push_str("<hr><p><em>(End of series)</em></p>");
            parts.push(current_part);
        }

        info!(total_parts = parts.len(), "split content into parts");
        parts
    }

    /// Create a series of Telegraph pages for large content.
    pub async fn create_page_series(&self, title: &str, html: &str) -> Result<Vec<String>> {
        let parts = self.split_content(html, title);
        let mut urls = Vec::new();
        let mut page_titles = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            let page_title = if i == 0 {
                title.to_string()
            } else {
                format!("{} (Part {})", title, i + 1)
            };

            match self.create_page(&page_title, part).await {
                Ok(url) => {
                    urls.push(url);
                    page_titles.push(page_title);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        part = i + 1,
                        total = parts.len(),
                        "failed to create page in series"
                    );
                }
            }

            if i < parts.len() - 1 {
                tokio::time::sleep(PAGE_CREATE_INTERVAL).await;
            }
        }

        if urls.is_empty() {
            return Err(anyhow::anyhow!("failed to create any pages in series"));
        }

        if urls.len() > 1 {
            tokio::time::sleep(PAGE_CREATE_INTERVAL * 2).await;

            let series_header = format_series_links(&urls);
            for (i, url) in urls.iter().enumerate() {
                let path = url.trim_start_matches("https://telegra.ph/");
                let new_html = format!("{}{}", series_header, parts[i]);

                if let Err(e) = self.edit_page(path, &page_titles[i], &new_html).await {
                    warn!(
                        error = %e,
                        page = i + 1,
                        "failed to add series links to page"
                    );
                }

                tokio::time::sleep(PAGE_CREATE_INTERVAL).await;
            }
        }

        info!(
            total_pages = urls.len(),
            title = %title,
            "created Telegraph page series"
        );

        Ok(urls)
    }
}
