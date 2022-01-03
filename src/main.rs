use std::{
    fmt::{Debug, Display},
    io::Write,
    path::PathBuf,
    process,
    str::FromStr,
};

use anyhow::Result;
use clap::{AppSettings, Parser};
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, CellAlignment, ContentArrangement, Table};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use strum::VariantNames;

use crate::{
    client::{Collection, List, Wishlist},
    config::Config,
    format::Format,
};

mod client;
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
    /// Overwrite existing values with the provided values
    #[clap(long, short, takes_value = false)]
    update: bool,
    /// Print the current configuration, other options are ignored
    #[clap(long, short, takes_value = false)]
    print: bool,
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

    // Configure and initialize the application logger. Logs to STDERR.
    configure_logger();

    // Load the configuration file if it has been created, or the default
    // configuration values if not.
    let config = Config::load()?;

    // If the application has not yet been configured or is misconfigured, and any
    // subcommand other than Configure is being run, instruct the user to first
    // configure the application.
    let opts = Opts::parse();
    if !matches!(opts.subcommand, Configure(..)) && !config.is_valid() {
        log::error!(
            "Missing or invalid configuration; please run `camper configure` and try again."
        );
        process::exit(1);
    }

    match opts.subcommand {
        Configure(opts) => configure(config, opts),
        List(opts) => list(config, opts).await,
        Download(opts) => download(config, opts).await,
        Sync(opts) => sync(config, opts).await,
    }
}

// ---------------------------------------------------------------------------
// Logging

fn configure_logger() {
    use env_logger::Builder;
    use log::LevelFilter;

    Builder::new()
        .format(|buf, record| {
            use env_logger::fmt::Color::*;
            use log::Level::*;

            let level = record.level();
            let mut level_style = buf.style();

            if level == Error {
                level_style.set_color(Red);
            } else if level == Warn {
                level_style.set_color(Yellow);
            }

            writeln!(
                buf,
                "⛺ {: <7} - {}",
                level_style.value(format!("[{}]", level)),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}

// ---------------------------------------------------------------------------
// Subcommands

fn configure(config: Config, opts: ConfigureOpts) -> Result<()> {
    if opts.print {
        eprintln!("{}", config);
    } else if opts.update {
        configure_update(config, opts)?;
    } else {
        configure_create(opts)?;
    }

    Ok(())
}

fn configure_create(opts: ConfigureOpts) -> Result<()> {
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
        log::error!("library path does not exist: '{}'", library.display());
        process::exit(1);
    }

    // Create and save the configuration to the config file location at
    // '~/.camper/config.toml'.
    let config = Config::new(fan_id, identity, library, format);
    config.save()?;

    Ok(())
}

fn configure_update(config: Config, opts: ConfigureOpts) -> Result<()> {
    let mut config = config;

    // For any provided options, update the corresponding value in the configuration
    // file, and upon successfully saving print a message to the user to alert that
    // this value has been updated.
    let mut messages = vec![];

    if let Some(fan_id) = opts.fan_id {
        messages.push(format!("Updated fan ID to {}\n", fan_id));
        config.fan_id = Some(fan_id);
    }
    if let Some(identity) = opts.identity {
        messages.push(format!("Updated identity to {}\n", identity));
        config.identity = Some(identity);
    }
    if let Some(library) = opts.library {
        let path = PathBuf::from(library);
        messages.push(format!("Updated library to {}\n", path.display()));
        config.library = Some(path);
    }
    if let Some(format) = opts.default_format {
        messages.push(format!("Updated default format to {}\n", format));
        config.format = Some(format);
    }

    config.save()?;
    for message in messages {
        log::info!("{}", message);
    }

    Ok(())
}

async fn list(config: Config, opts: ListOpts) -> Result<()> {
    // A fan ID can optionally be provided to list their collection(s) instead. By
    // default, the configured fan ID will be used.
    let fan_id = opts.fan_id.or(config.fan_id).unwrap();
    let identity = config.identity.unwrap();

    // Query all items from the specified collection. We make authenticated requests
    // here to show any private or hidden items when listing the authenticated users
    // collection(s).
    let items = if opts.wishlist {
        Wishlist::list(fan_id, &identity).await?
    } else {
        Collection::list(fan_id, &identity).await?
    };
    let total_items = items.len();

    // Print the list of collection items in a tabular format for easy grokking.
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Album ID")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
            Cell::new("Band").add_attribute(Attribute::Bold),
            Cell::new("Album Title").add_attribute(Attribute::Bold),
        ]);

    for item in items {
        table.add_row(vec![
            Cell::new(item.album_id).set_alignment(CellAlignment::Right),
            Cell::new(item.band_name),
            Cell::new(item.album_title),
        ]);
    }

    eprintln!("{}", table);
    eprintln!("\n{} items\n", total_items);

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
