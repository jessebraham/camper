use anyhow::Result;
use async_trait::async_trait;
use indicatif::{ProgressBar, ProgressStyle};

use self::query::{utc_now_token, QueryBuilder, QueryItem};

mod query;

// ---------------------------------------------------------------------------
// Traits

#[async_trait]
pub trait List {
    const COLLECTION_NAME: &'static str;

    async fn list(fan_id: u32, identity: &str) -> Result<Vec<QueryItem>> {
        let mut items = vec![];

        // Create a progress spinner to indicate to the user that something is indeed
        // happening, as this process can take some time depending on collection size
        // and connection speed.
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(80);
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["🌑 ", "🌒 ", "🌓 ", "🌔 ", "🌕 ", "🌖 ", "🌗 ", "🌘 "])
                .template("{spinner} {msg}"),
        );
        pb.set_message(format!("Loading {} items...", Self::COLLECTION_NAME));

        // Repeatedly query the URL until no more results are available. Collect all
        // results into a vector, which we will ultimately return.
        let url = format!(
            "https://bandcamp.com/api/fancollection/1/{}_items",
            Self::COLLECTION_NAME
        );

        let mut token = utc_now_token();
        loop {
            let response = QueryBuilder::new(&url)
                .fan_id(fan_id)
                .identity(&identity)
                .token(&token)
                .query()
                .await?;

            items.extend(response.items);
            token = response.last_token;

            if !response.more_available {
                pb.finish_and_clear();
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
    const COLLECTION_NAME: &'static str = "collection";
}

// ---------------------------------------------------------------------------
// Wishlist

pub struct Wishlist;

#[async_trait]
impl List for Wishlist {
    const COLLECTION_NAME: &'static str = "wishlist";
}
