use crate::{
    constants,
    log_parser::{parse_log, SiteLog},
};
use anyhow::{Context, Result};
use async_std::{stream::StreamExt, task::JoinHandle};
use serde_json::json;
use std::{path::PathBuf, str::FromStr, time::Duration};
use tracing::debug;

use chromiumoxide::{browser::Browser, error::CdpError, BrowserConfig};

/// Wrapper around a Browser instance that takes are of polling the handler
/// and holds additional options.
pub struct Crawler {
    browser: Browser,
    handle: JoinHandle<()>,
    page_timeout: u64,
}

impl Crawler {
    /// Create a new chromium process with the given settings
    pub async fn new(config: BrowserConfig, page_timeout: u64) -> Result<Self> {
        // create a `Browser` that spawns a chromium process
        // and the handler that drives the websocket etc.
        let (browser, mut handler) = Browser::launch(config)
            .await
            .context("failed to launcher browser")?;

        // spawn a new task that continuously polls the handler
        let handle = async_std::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            browser,
            handle,
            page_timeout,
        })
    }

    /// Stop this chromium process
    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        self.handle.await;
        Ok(())
    }

    /// Visit a URL in a new tab and capture its log from ghunter
    pub async fn visit_url(&self, url: &str, on_load_script: Option<&str>) -> Result<SiteLog> {
        let page = self.browser.new_page("about:blank").await?;
        page.wait_for_navigation().await?;
        page.enable_stealth_mode_with_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36").await?;
        page.evaluate_on_new_document("__ghunter_log('DOCUMENT_LOAD');")
            .await?;
        if let Some(on_load_script) = on_load_script {
            page.evaluate_on_new_document(on_load_script).await?;
        }
        let log_path: String = page
            .evaluate_function("__ghunter_getLogPath")
            .await?
            .into_value()?;

        match page.goto(url).await.map(|_| ()) {
            // some pages get stuck on goto for some reason, even though they loaded fine
            Err(CdpError::Timeout) => {}
            result => result?,
        }
        page.wait_for_navigation().await?;

        // wait for 5 seconds allow for potential events to happen
        std::thread::sleep(Duration::from_secs(self.page_timeout));

        page.close().await?;

        let log_path = PathBuf::from_str(&log_path)
            .with_context(|| format!("failed while opening log file at {}", &log_path))?;
        let log = parse_log(&log_path)
            .with_context(|| format!("failed while parsing log file at {:?}", &log_path))?;

        debug!("page log: {:#?}", log);

        Ok(log)
    }
}

/// Generate a script, to be loaded at the start of each frame,
/// that pollutes the given properties in the object prototype,
/// setting their values to a taint tracker (with some additional data
/// dynamically added through a getter).
pub fn gen_polluting_script(properties_to_pollute: &[(String, Option<String>)]) -> String {
    let polluted_marker = json!(constants::POLLUTED_MARKER);
    let properties_json = json!(properties_to_pollute);

    format!(
        r#"
const __ghunter = {{
  pollutedMarker: {}, // <-- this is dynamic
  propertiesToPollute: {}, // <-- this is dynamic
}};

__ghunter.propertiesToPollute.forEach(([p, v], i) => {{
  if (v !== null && v !== undefined) {{
    Object.prototype[p] = v;
    return;
  }}
  let accessIndex = 0;
  Object.defineProperty(
    Object.prototype,
    p,
    {{
      get: function() {{
        const returnValue = `${{__ghunter.pollutedMarker}}:${{i}}:${{accessIndex}}`;
        accessIndex += 1;

        try {{
          throw new Error();
        }} catch (error) {{
          const stacktrace = error.stack;
          __ghunter_log(`PROTOTYPE_GET ${{p.length}} ${{p}} ${{returnValue.length}} ${{returnValue}} ${{stacktrace.length}} ${{stacktrace}}`);
        }}

        // TODO: support proxy
        return returnValue;
      }},
      set: function(newValue) {{
        Object.defineProperty(
          this,
          p,
          {{
            value: newValue,
            writable: true,
            enumerable: true,
            configurable: true
          }}
        );
      }},
      enumerable: true,
      configurable: true
    }}
  );
}})"#,
        polluted_marker, properties_json
    )
}