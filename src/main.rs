mod config;
mod runner;

use clap::Parser;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Setup logging with ANSI color support
    let filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .init();

    let args = config::AppArgs::parse();

    info!("Starting Pax - SSH SOCKS5 Proxy");

    // Check mode for initial log
    if args.host.is_some() {
        info!("Mode: CLI Arguments (Target: {})", args.host.as_ref().unwrap());
    } else {
        info!("Mode: API Fetch (Target: {})", args.api);
    }

    loop {
        if let Err(e) = run_session(&args).await {
            error!("Session ended: {:?}", e);

            // Critical Fix: If interrupted by user (Ctrl+C), exit the process immediately.
            if e.to_string().contains("Interrupted by user") {
                info!("Exiting immediately.");
                std::process::exit(0);
            }
        }

        info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_session(args: &config::AppArgs) -> anyhow::Result<()> {
    // 1. Determine Source (CLI vs API)
    let mut ssh_cfg = if args.host.is_some() {
        // CLI Mode
        config::create_from_args(args)?
    } else {
        // API Mode
        config::fetch_ssh_config(&args.api, args.timeout).await?
    };

    // 2. Override for API mode (Optional mixing)
    // If user provides -k in API mode, we still want to respect it.
    // In CLI mode, create_from_args already handled this, but re-assigning is harmless.
    if args.host.is_none() {
         if let Some(ref local_key_path) = args.private_key {
            info!("Overriding auth: Using local private key -> {}", local_key_path);
            ssh_cfg.auth_type = config::AuthType::Key;
            ssh_cfg.private_key = Some(local_key_path.clone());
        }
    }

    // 3. Display Config (Unified visualization)
    config::print_node_info(&ssh_cfg);

    // 4. Prepare Private Key (Temp file or Local path)
    let _key_guard: Option<tempfile::NamedTempFile>;

    if ssh_cfg.auth_type == config::AuthType::Key {
        if let Some(ref raw_key) = ssh_cfg.private_key {
            let (final_path, guard) = config::prepare_private_key(raw_key)?;
            ssh_cfg.private_key = Some(final_path);
            _key_guard = guard;
        } else {
            return Err(anyhow::anyhow!("AuthType is Key but no key provided."));
        }
    } else {
        _key_guard = None;
    }

    let port = args.local_port;
    let host = args.local_host.clone();
    let cfg_clone = ssh_cfg.clone();

    // 5. Run SSH with Signal Handling
    tokio::select! {
        res = tokio::task::spawn_blocking(move || {
            runner::start_ssh_process(&host, port, &cfg_clone)

        }) => {
            match res {
                Ok(inner) => inner,
                Err(e) => Err(anyhow::anyhow!("Join error: {}", e)),
            }
        }
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, cleaning up...");
            Err(anyhow::anyhow!("Interrupted by user"))
        }
    }
}
