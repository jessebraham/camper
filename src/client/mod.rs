use anyhow::Result;
use async_trait::async_trait;

use self::request::{utc_now_token, QueryBuilder, QueryItem};

mod request;

// ---------------------------------------------------------------------------
// Traits

#[async_trait]
pub trait List {
    const QUERY_URL: &'static str;

    async fn list(fan_id: u32) -> Result<Vec<QueryItem>> {
        let mut items = vec![];

        let mut token = utc_now_token();
        loop {
            let response = QueryBuilder::new(Self::QUERY_URL)
                .fan_id(fan_id)
                .token(&token)
                .query()
                .await?;

            items.extend(response.items);
            token = response.last_token;

            if !response.more_available {
                break;
            }
        }

        Ok(items)
    }
}

// ---------------------------------------------------------------------------
// Collection

pub struct Collection;

#[async_trait]
impl List for Collection {
    const QUERY_URL: &'static str = "https://bandcamp.com/api/fancollection/1/collection_items";
}

// ---------------------------------------------------------------------------
// Wishlist

pub struct Wishlist;

#[async_trait]
impl List for Wishlist {
    const QUERY_URL: &'static str = "https://bandcamp.com/api/fancollection/1/wishlist_items";
}
