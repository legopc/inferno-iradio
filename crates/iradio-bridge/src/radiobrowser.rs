use crate::state::Station;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

const RADIOBROWSER_DNS: &str = "all.api.radio-browser.info";

#[derive(Debug, Deserialize)]
struct RbStation {
    stationuuid: String,
    name: String,
    url_resolved: String,
    codec: String,
    bitrate: u32,
    countrycode: String,
    tags: String,
    favicon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub stationcount: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    pub iso_3166_1: String,
    pub stationcount: u32,
}

#[derive(Clone)]
pub struct RadioBrowserClient {
    client: Client,
    base_url: String,
}

impl RadioBrowserClient {
    pub async fn new(configured_url: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("inferno-iradio/0.1 (+https://github.com/legopc/inferno-iradio)")
            .build()
            .unwrap_or_default();

        let base_url = if !configured_url.is_empty() {
            configured_url.to_string()
        } else {
            Self::resolve_api_url(&client).await
        };

        debug!("RadioBrowser API base: {}", base_url);
        Self { client, base_url }
    }

    async fn resolve_api_url(client: &Client) -> String {
        // Try DNS resolution of all.api.radio-browser.info
        match tokio::net::lookup_host(format!("{}:443", RADIOBROWSER_DNS)).await {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    // Use the IP to avoid DNS lookup on every request
                    return format!("https://{}/json", addr.ip());
                }
            }
            Err(e) => {
                tracing::warn!("RadioBrowser DNS lookup failed: {}", e);
            }
        }
        // Fallback to well-known server
        "https://de1.api.radio-browser.info/json".to_string()
    }

    fn map_station(s: RbStation) -> Station {
        Station {
            id: s.stationuuid,
            name: s.name,
            url: s.url_resolved,
            codec: s.codec,
            bitrate: s.bitrate,
            country: s.countrycode,
            tags: s
                .tags
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
            favicon: s.favicon,
        }
    }

    pub async fn search(
        &self,
        q: &str,
        country: Option<&str>,
        tag: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<Station>> {
        let mut params = vec![
            ("name", q.to_string()),
            ("limit", limit.to_string()),
            ("order", "clickcount".to_string()),
            ("reverse", "true".to_string()),
            ("hidebroken", "true".to_string()),
        ];
        if let Some(c) = country {
            params.push(("countrycode", c.to_string()));
        }
        if let Some(t) = tag {
            params.push(("tag", t.to_string()));
        }

        let url = format!("{}/stations/search", self.base_url);
        let resp: Vec<RbStation> = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.into_iter().map(Self::map_station).collect())
    }

    pub async fn top(&self, limit: usize) -> anyhow::Result<Vec<Station>> {
        let url = format!("{}/stations/topclick/{}", self.base_url, limit);
        let resp: Vec<RbStation> = self
            .client
            .get(&url)
            .query(&[("hidebroken", "true")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.into_iter().map(Self::map_station).collect())
    }

    pub async fn tags(&self, limit: usize) -> anyhow::Result<Vec<Tag>> {
        let url = format!("{}/tags", self.base_url);
        let resp: Vec<Tag> = self
            .client
            .get(&url)
            .query(&[
                ("order", "stationcount"),
                ("reverse", "true"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    pub async fn countries(&self) -> anyhow::Result<Vec<Country>> {
        let url = format!("{}/countries", self.base_url);
        let resp: Vec<Country> = self
            .client
            .get(&url)
            .query(&[("order", "stationcount"), ("reverse", "true")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    pub async fn stations_by_tag(&self, tag: &str, limit: usize) -> anyhow::Result<Vec<Station>> {
        let url = format!(
            "{}/stations/bytag/{}",
            self.base_url,
            urlencoding::encode(tag)
        );
        let limit_s = limit.to_string();
        let resp: Vec<RbStation> = self
            .client
            .get(&url)
            .query(&[
                ("limit", limit_s.as_str()),
                ("order", "clickcount"),
                ("reverse", "true"),
                ("hidebroken", "true"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.into_iter().map(Self::map_station).collect())
    }

    pub async fn stations_by_country(
        &self,
        country_code: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<Station>> {
        let url = format!(
            "{}/stations/bycountrycodeexact/{}",
            self.base_url, country_code
        );
        let limit_s = limit.to_string();
        let resp: Vec<RbStation> = self
            .client
            .get(&url)
            .query(&[
                ("limit", limit_s.as_str()),
                ("order", "clickcount"),
                ("reverse", "true"),
                ("hidebroken", "true"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.into_iter().map(Self::map_station).collect())
    }
}
