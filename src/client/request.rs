use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};

// ---------------------------------------------------------------------------
// Request & Response Data

/// The required payload for listing items in a collection or wishlist.
#[derive(Debug, Serialize)]
pub struct QueryRequestData {
    fan_id: u32,
    #[serde(rename = "older_than_token")]
    token: String,
    count: u32,
}

impl QueryRequestData {
    // Increasing this value seems to cause issues, so until that problem is
    // resolved we will just default to what the Bandcamp website uses.
    const DEFAULT_COUNT: u32 = 20;

    fn new(fan_id: u32, token: String) -> Self {
        Self {
            fan_id,
            token,
            count: Self::DEFAULT_COUNT,
        }
    }
}

/// The data returned from the API call to list collection or wishlist items.
#[derive(Debug, Deserialize)]
pub struct QueryResponseData {
    pub items: Vec<QueryItem>,
    pub last_token: String,
    pub more_available: bool,
}

// ---------------------------------------------------------------------------
// Query Item

/// A singular item contained within the collection or wishlist; usually an
/// album, but sometimes tracks.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize)]
pub struct QueryItem {
    #[serde(deserialize_with = "deserialize_rfc2822_datetime")]
    pub added: DateTime<Utc>,
    pub band_name: String,
    pub album_id: u32,
    pub album_title: String,
}

fn deserialize_rfc2822_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let utc = DateTime::parse_from_rfc2822(&buf)
        .unwrap()
        .with_timezone(&Utc);

    Ok(utc)
}

// ---------------------------------------------------------------------------
// Query Builder

#[derive(Debug, Default, Clone, Copy)]
pub struct QueryBuilder<'a> {
    url: &'a str,
    fan_id: u32,
    token: Option<&'a str>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(url: &'a str) -> Self {
        let mut me = Self::default();
        me.url = url;

        me
    }

    pub fn fan_id(&mut self, fan_id: u32) -> &mut Self {
        self.fan_id = fan_id;

        self
    }

    pub fn token(&mut self, token: &'a str) -> &mut Self {
        self.token = Some(token);

        self
    }

    pub async fn query(self) -> Result<QueryResponseData> {
        let token = match self.token {
            Some(token) => token.to_string(),
            None => utc_now_token(),
        };

        let response = Client::new()
            .post(self.url)
            .json(&QueryRequestData::new(self.fan_id, token))
            .send()
            .await?
            .json::<QueryResponseData>()
            .await?;

        Ok(response)
    }
}

pub fn utc_now_token() -> String {
    format!("{}:0:a::", Utc::now().timestamp())
}
