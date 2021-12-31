use std::{
    fmt::{Debug, Display},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{bail, Result};
use clap::{AppSettings, Parser};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use strum::VariantNames;

use crate::{config::Config, format::Format};

mod config;
mod format;

// ---------------------------------------------------------------------------
// Command-Line Application

#[derive(Debug, Parser)]
#[clap(about, version)]
#[clap(global_setting = AppSettings::PropagateVersion)]
struct Opts {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Configure the application with authentication and library settings
    Configure(ConfigureOpts),
    /// List all albums in a collection or wishlist
    List(ListOpts),
    /// Download one or more albums from a collection
    Download(DownloadOpts),
    /// Synchronize a directory with a collection
    Sync(SyncOpts),
}

#[derive(Debug, Parser)]
struct ConfigureOpts {
    /// Bandcamp user identifier
    #[clap(long, short)]
    fan_id: Option<u32>,
    /// Bandcamp identity cookie
    #[clap(long, short)]
    identity: Option<String>,
    /// Path to music library
    #[clap(long, short)]
    library: Option<String>,
    /// Default audio file format to download
    #[clap(long, short, possible_values = Format::VARIANTS)]
    default_format: Option<Format>,
}

#[derive(Debug, Parser)]
struct ListOpts {
    /// ID of the user whose collection items to list
    #[clap(long, short)]
    fan_id: Option<u32>,
    /// List items from the wishlist instead
    #[clap(long, short, takes_value = false)]
    wishlist: bool,
}

#[derive(Debug, Parser)]
struct DownloadOpts {
    /// File format to download albums in
    #[clap(long, short, possible_values = Format::VARIANTS)]
    format: Option<Format>,
    /// One or more album IDs to download
    #[clap(required = true)]
    album_ids: Vec<u32>,
}

#[derive(Debug, Parser)]
struct SyncOpts {
    /// File format to download albums in
    #[clap(long, short, possible_values = Format::VARIANTS)]
    format: Option<Format>,
    /// Directory to sync albums to
    #[clap(required = true)]
    directory: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    use Subcommand::*;

    // Load the configuration file if it has been created, or the default
    // configuration values if not.
    let config = Config::load()?;

    // If the application has not yet been configured or is misconfigured, and any
    // subcommand other than Configure is being run, instruct the user to first
    // configure the application.
    let opts = Opts::parse();
    if !matches!(opts.subcommand, Configure(..)) && !config.is_valid() {
        bail!("please run `camper configure` first");
    }

    match opts.subcommand {
        Configure(opts) => configure(opts),
        List(opts) => list(config, opts).await,
        Download(opts) => download(config, opts).await,
        Sync(opts) => sync(config, opts).await,
    }
}

// ---------------------------------------------------------------------------
// Subcommands

fn configure(opts: ConfigureOpts) -> Result<()> {
    // For each required piece of information, if it was provided as a command-line
    // argument use that value, otherwise prompt the user to enter a value.
    let fan_id = unwrap_or_prompt(opts.fan_id, "Bandcamp fan ID");
    let identity = unwrap_or_prompt(opts.identity, "Bandcamp identity cookie");
    let library = unwrap_or_prompt(opts.library, "Music library directory");

    // The default file format is handled much the same as above, but we provide the
    // enumerated formats which are allowed instead of allowing arbitrary input.
    let format = opts.default_format.unwrap_or_else(|| {
        let index = Select::with_theme(&ColorfulTheme::default())
            .items(&Format::VARIANTS)
            .default(0)
            .interact()
            .unwrap();

        Format::from_repr(index).unwrap()
    });

    // Verify that the library path does indeed exist.
    let library = PathBuf::from(library).canonicalize()?;
    if !library.exists() {
        bail!("path does not exist: '{}'", library.display());
    }

    // Create and save the configuration to the config file location at
    // '~/.camper/config.toml'.
    let config = Config::new(fan_id, identity, library, format);
    config.save()?;

    Ok(())
}

async fn list(_config: Config, _opts: ListOpts) -> Result<()> {
    Ok(())
}

async fn download(_config: Config, _opts: DownloadOpts) -> Result<()> {
    Ok(())
}

async fn sync(_config: Config, _opts: SyncOpts) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper Functions

/// Return either the inner value of the provided Option `opt`, or the user's
/// response to `prompt`.
fn unwrap_or_prompt<T>(opt: Option<T>, prompt: &str) -> T
where
    T: Clone + Display + FromStr,
    <T as FromStr>::Err: Debug + Display,
{
    opt.unwrap_or_else(|| {
        Input::<T>::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    })
}