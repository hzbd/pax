use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use clap::Parser;
use colored::*;
use reqwest::Client;
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::NamedTempFile;
use tracing::info;

#[derive(Parser, Debug, Clone)]
pub struct AppArgs {
    // --- Mode 1: API Configuration ---

    /// API endpoint URL (Used if --host is not provided)
    #[arg(long, env = "PAX_API_URL", default_value = "https://example.com/api/auth.json")]
    pub api: String,

    /// Request timeout in seconds (for API fetch)
    #[arg(long, default_value = "10")]
    pub timeout: u64,

    // --- Mode 2: Manual Configuration (CLI) ---

    /// Remote Server Host / IP (Enables CLI Mode, ignores API)
    #[arg(long)]
    pub host: Option<String>,

    /// Remote SSH User
    #[arg(long)]
    pub user: Option<String>,

    /// Remote SSH Port
    #[arg(long, default_value = "22")]
    pub ssh_port: String,

    /// SSH Password
    #[arg(long)]
    pub password: Option<String>,

    /// Private key file path (Used for both API override and CLI mode)
    #[arg(short = 'k', long)]
    pub private_key: Option<String>,

    // --- Common Settings ---

    /// Local SOCKS5 port
    #[arg(short, long, default_value = "1080")]
    pub local_port: u16,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Password,
    Key,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SshConfig {
    pub user: String,
    pub host: String,

    #[serde(default = "default_port")]
    pub port: String,

    #[serde(default = "default_auth_type")]
    pub auth_type: AuthType,

    // Optional metadata
    pub region: Option<String>,

    #[serde(rename = "ref")]
    pub ref_info: Option<String>,

    pub password: Option<String>,
    pub private_key: Option<String>,

    pub exp_at: Option<String>,
}

fn default_port() -> String { "22".to_string() }
fn default_auth_type() -> AuthType { AuthType::Password }

/// Helper: Prints the node information visually.
pub fn print_node_info(config: &SshConfig) {
    let region_display = config.region.as_deref().unwrap_or("UNK");

    println!();
    println!("{} {}@{}:{}",
        "  -> Node:".bold(),
        config.user.green(),
        config.host.green(),
        config.port.yellow()
    );

    println!("{} {} ({:?})",
        "  -> Info:".bold(),
        region_display.cyan(),
        config.auth_type
    );

    if let Some(ref r) = config.ref_info {
        println!("{} {}", "  -> Ref :".bold(), r.blue().underline());
    }
    println!();

    check_expiration(&config.exp_at);
}

/// Creates SshConfig directly from CLI arguments.
pub fn create_from_args(args: &AppArgs) -> Result<SshConfig> {
    let host = args.host.clone().ok_or_else(|| anyhow!("Host is required in CLI mode"))?;
    let user = args.user.clone().unwrap_or_else(|| "root".to_string());

    let auth_type = if args.private_key.is_some() {
        AuthType::Key
    } else {
        AuthType::Password
    };

    let config = SshConfig {
        user,
        host,
        port: args.ssh_port.clone(),
        auth_type,
        region: Some("Local".to_string()),
        ref_info: Some("CLI Args".to_string()),
        password: args.password.clone(),
        private_key: args.private_key.clone(),
        exp_at: None,
    };

    Ok(config)
}

/// Fetches and parses SSH config from the API.
pub async fn fetch_ssh_config(api_url: &str, timeout_secs: u64) -> Result<SshConfig> {
    info!("Fetching credentials...");

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let resp = client.get(api_url).send().await.context("API request failed")?;
    let text = resp.text().await.context("Failed to get response text")?;

    let config: SshConfig = match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to parse JSON. Raw response content:\n{}", text);
            return Err(anyhow::anyhow!("JSON parse error: {}", e));
        }
    };

    Ok(config)
}

fn expand_tilde(path_str: &str) -> PathBuf {
    if path_str.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            if path_str == "~" {
                return home;
            }
            if path_str.starts_with("~/") || path_str.starts_with("~\\") {
                 return home.join(&path_str[2..]);
            }
        }
    }
    PathBuf::from(path_str)
}

pub fn prepare_private_key(key_input: &str) -> Result<(String, Option<NamedTempFile>)> {
    if key_input.contains("PRIVATE KEY") {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(key_input.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = temp_file.as_file().metadata()?.permissions();
            perms.set_mode(0o600);
            temp_file.as_file().set_permissions(perms)?;
        }

        let path = temp_file.path().to_string_lossy().to_string();
        Ok((path, Some(temp_file)))
    } else {
        let expanded_path = expand_tilde(key_input);

        if expanded_path.exists() && expanded_path.is_file() {
            Ok((expanded_path.to_string_lossy().to_string(), None))
        } else {
            Err(anyhow!("Private key file not found: {} (Expanded: {:?})", key_input, expanded_path))
        }
    }
}

fn parse_flexible_date(date_str: &str) -> Option<NaiveDateTime> {
    let formats = [
        "%Y-%m-%d / %H:%M:%S", "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S", "%Y/%m/%d %H:%M:%S",
    ];

    for fmt in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, fmt) {
            return Some(dt);
        }
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.naive_local());
    }
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return date.and_hms_opt(23, 59, 59);
    }
    None
}

fn check_expiration(exp_at: &Option<String>) {
    let date_str = match exp_at {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };

    match parse_flexible_date(date_str) {
        Some(expire_dt) => {
            let now = Local::now().naive_local();
            let hours_left = (expire_dt - now).num_hours();

            if hours_left < 0 {
                println!("\n{}\n", "!!! ACCOUNT EXPIRED !!!".on_red().white().bold());
                println!("Expired at: {}", date_str.red());
            } else if hours_left < 24 {
                println!("\n{}", "==========================================".yellow());
                println!("{} {}", "!!! WARNING: EXPIRING SOON !!!".red().bold(), "(< 24h)".yellow());
                println!("Remaining: {} hours (Until: {})", hours_left.to_string().red().bold(), date_str);
                println!("{}", "==========================================\n".yellow());
            } else {
                println!("  -> Valid until: {}", date_str.green());
            }
        },
        None => tracing::warn!("Unknown date format: {}", date_str),
    }
}
