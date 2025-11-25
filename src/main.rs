use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use constants::POLLUTED_MARKER;
use crawler::{gen_polluting_script, Crawler};
use log_parser::{LogMessage, SiteLog};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use tempdir::TempDir;
use tracing::{error, info};

use chromiumoxide::browser::{BrowserConfig, HeadlessMode};

mod constants;
mod crawler;
mod log_parser;

/// Find client-side prototype pollution gadgets in websites
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// URL (including schema) to visit and search for undefined properties and sinks.
    /// Can be provided multiple times.
    #[arg(short, long, group = "urls")]
    url: Vec<String>,

    /// File containing URLs to visit and search for undefined properties and sinks.
    ///
    /// Expected format is one URL (including schema, e.g., https://) per line.
    /// Lines containing only whitespace are ignored.
    /// Lines starting with # are treated as comments and thus ignored as well.
    #[arg(short = 'f', long, group = "urls")]
    url_file: Option<PathBuf>,

    /// Path to the ghunter4chrome chromium executable. If not provided, the binary named
    /// `chromium-ghunter` present in PATH will be used.
    #[arg(long, env = "CHROMIUM_GHUNTER_EXECUTABLE")]
    chromium_executable: Option<PathBuf>,

    /// When set, the browser will be launched graphically instead of running in headless mode.
    #[arg(short = 'g', long)]
    headful: bool,

    /// How many seconds to wait after a page has loaded before proceeding.
    #[arg(short = 't', long, default_value_t = 5)]
    page_timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all undefined properties in the webpage
    Undefined,

    /// Pollute the prototype with a property containing the taint marker to
    /// detect enumerable properties.
    Enumerable,

    /// Pollute the given properties.
    Custom {
        /// The name (and possibly value, separated by =) of the property to pollute.
        /// Can be given multiple times.
        ///
        /// The value of the property will be a taint marker if not provided.
        #[arg(short = 'p', long = "property", value_parser = Commands::parse_custom_property, value_name = "KEY[=VALUE]")]
        properties: Vec<(String, Option<String>)>,
    },
}

impl Commands {
    async fn run_action(&self, crawler: &Crawler, url: &str) -> Result<()> {
        match self {
            Commands::Undefined => find_website_undefined_properties(crawler, url).await,
            Commands::Enumerable => {
                find_sinks_from_custom_properties(
                    crawler,
                    url,
                    &[(POLLUTED_MARKER.to_owned(), None)],
                )
                .await
            }
            Commands::Custom { properties } => {
                find_sinks_from_custom_properties(crawler, url, properties).await
            }
        }
    }
    fn parse_custom_property(s: &str) -> Result<(String, Option<String>), String> {
        if let Some((key, value)) = s.split_once('=') {
            Ok((key.to_string(), Some(value.to_string())))
        } else {
            Ok((s.to_string(), None))
        }
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let urls = if let Some(url_file) = cli.url_file {
        let file = File::open(url_file)?;
        let buf_reader = BufReader::new(file);

        buf_reader
            .lines()
            .map_while(Result::ok)
            .filter(|s| !s.trim().is_empty())
            .filter(|s| !s.trim_start().starts_with("#"))
            .collect()
    } else {
        cli.url
    };

    if urls.is_empty() {
        bail!("no urls have been provided");
    }

    info!("processing {} url(s)", urls.len());

    let executable = match cli.chromium_executable {
        Some(path) => path,
        None => which::which("chromium-ghunter")
            .context("failed to get chromium-ghunter executable in PATH")?,
    };

    let user_data_dir = TempDir::new("ghunter4chrome-gadget-finder")
        .context("failed to create temporary directory for browser user data")?;

    let headless_mode = if cli.headful {
        HeadlessMode::False
    } else {
        HeadlessMode::New
    };

    let browser_config = BrowserConfig::builder()
        .chrome_executable(executable)
        .user_data_dir(user_data_dir)
        .headless_mode(headless_mode)
        .build()
        .map_err(|e| anyhow!(e))?;

    let crawler = Crawler::new(browser_config, cli.page_timeout)
        .await
        .context("failed to create crawler instance")?;

    for url in urls {
        info!(url = url, "visiting website");
        if let Err(error) = cli.command.run_action(&crawler, &url).await {
            error!(url = url, "failed while visiting website {:?}", error);
        };
    }

    crawler
        .close()
        .await
        .context("failed to close the crawler instance")?;
    Ok(())
}

async fn find_website_undefined_properties(crawler: &Crawler, url: &str) -> Result<()> {
    let log = crawler.visit_url(url, None).await?;

    let undefined_properties = get_all_undefined_properties(&log);

    info!("found {} undefined properties", undefined_properties.len());
    for name in undefined_properties {
        info!(name = name, "undefined property");
    }

    Ok(())
}

fn get_all_undefined_properties(log: &SiteLog) -> BTreeSet<String> {
    // we only care about the log when it opens the actual page, so skip messages until a
    // frame location is logged
    let mut undefined_properties = BTreeSet::new();
    for msg in log
        .messages
        .iter()
        .skip_while(|msg| !matches!(msg, log_parser::LogMessage::DocumentStart))
    {
        if let LogMessage::UndefinedProperty { name, .. } = msg {
            undefined_properties.insert(name.clone());
        }
    }

    undefined_properties
}

async fn find_sinks_from_custom_properties(
    crawler: &Crawler,
    url: &str,
    properties: &[(String, Option<String>)],
) -> Result<()> {
    let polluting_script = gen_polluting_script(properties);
    let log = crawler.visit_url(url, Some(&polluting_script)).await?;

    let relevant_log_entries = retain_sink_related_log_entries(&log);

    info!("found {} relevant log entries", relevant_log_entries.len());
    for entry in relevant_log_entries {
        info!("log entry {:#?}", entry);
    }

    Ok(())
}

fn retain_sink_related_log_entries(log: &SiteLog) -> Vec<&LogMessage> {
    log.messages
        .iter()
        .filter(|msg| match msg {
            LogMessage::AssignTaintedKey { class_name, .. } => {
                !matches!(class_name.as_str(), "Object" | "Array" | "Function")
            }
            LogMessage::SinkReached { .. } => true,
            _ => false,
        })
        .collect()
}