# Pax - Automated SSH SOCKS5 Proxy

Pax is a lightweight Rust tool designed to **replace manual SSH SOCKS5 commands** (e.g., `ssh -D 1080 ...`). It can fetch credentials dynamically from a remote API **OR** take them directly from the command line, establishing the tunnel and keeping it alive automatically.

```mermaid
flowchart TD
    %% Control Plane
    subgraph Control ["Control Plane (Pax Core Engine)"]
        direction TB
        S1[/"Step 1: Configuration Resolution\n(Fetch JSON API or Read CLI Args)"/]
        S2["Step 2: Credential Preparation\n(Hold passwords or write temp keys to disk)"]
        S3{"Step 3: Spawn & Auto-Auth\n(Run 'ssh -D -N', auto-inject passwords via expectrl)"}
        S5(("Step 5: Monitor & Self-Healing\n(Detect EOF/Crash, wait 5s and reconnect)"))

        S1 --> S2 --> S3
        S3 -. "Connection drops\n(Timeout / Refused)" .-> S5
        S5 -. "Loop back to start" .-> S1
    end

    %% Data Plane
    subgraph Data ["Data Plane (Traffic Forwarding)"]
        direction LR
        Apps("Client Apps\n(Browser / Telegram)")
        SSHProcess["Step 4: Local OpenSSH Process\n(Listening on 127.0.0.1:1080)"]
        Server["Remote SSH Server\n(Port 22)"]
        Internet(("Public Internet"))

        Apps -- "SOCKS5 Proxy" --> SSHProcess
        SSHProcess == "Encrypted SSH Tunnel" ==> Server
        Server -- "Forwarded Traffic" --> Internet
    end

    %% Tie Control and Data planes together
    S3 == "Spawns process & \nmanages lifecycle" === SSHProcess

    %% Styling
    classDef step fill:#fff3e0,stroke:#ffb74d,stroke-width:2px,color:#000
    classDef network fill:#e1f5fe,stroke:#4fc3f7,stroke-width:2px,color:#000
    classDef external fill:#f5f5f5,stroke:#9e9e9e,stroke-width:2px,color:#000
    
    class S1,S2,S3,S5 step
    class SSHProcess,Server network
    class Apps,Internet external
    
    %% Dashed border for Pax Core Engine
    style Control fill:transparent,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
```

## Why use Pax?

| **Feature** | **Pax** |
| :--- | :--- |
| **Stability** | Detects disconnects (Timeout/EOF) and **auto-reconnects**. Gracefully handles `Ctrl+C` exits. |
| **Dynamic** | Fetches credentials from a JSON API (Auto-rotate / List parsing). |
| **Manual** | Can also act as a robust `autossh` alternative via CLI args. |
| **Keys** | Handles **Local Keys** (`~/.ssh/...`) & **Raw Key Content**. |

## Features

*   **Versatile Config**: Supports both **Remote API** fetching and **Manual CLI** arguments.
*   **Environment Friendly**: Supports configuration via **Environment Variables** (`PAX_API_URL`).
*   **Silent Mode Support**: Compatible with SSH servers that suppress output (`-N` mode), automatically detecting successful connections.
*   **Expiration Aware**: Visual alerts if the account is expiring soon (<24h).
*   **Smart Paths**: Automatically expands `~` to your home directory for private key paths.
*   **Metadata Display**: Shows server **Region** and **Source Ref** for better tracking.

![State](./screenshot.png)
*Display under normal conditions.*

![State with datetime exp notify](./screenshot_notify.png)
*A prominent reminder appears on startup when authentication is about to expire.*

![State with cli](./screenshot_local.png)
*Display under normal conditions with local args*

## Usage

### 1. Build & Run
```bash
cargo build --release
./target/release/pax
```

### 2. Configuration Modes

Pax supports two primary modes. If `--host` is provided, it switches to **CLI Mode**. Otherwise, it defaults to **API Mode**.

#### Mode A: API Driven (Default)
Recommended for managing many dynamic servers.

```bash
# Basic (uses default API URL or Env Var)
./pax

# Custom API URL
./pax --api "https://my-api.com/nodes"

# API + Local Key Override
./pax --api "..." -k "~/.ssh/id_rsa"
```

#### Mode B: CLI Arguments (Manual)
Recommended for single servers or replacing `ssh -D`.

```bash
# Password Auth
./pax --host 1.2.3.4 --user root --password "secret123"

# Key Auth
./pax --host 1.2.3.4 --user root -k "~/.ssh/id_rsa"

# Custom Ports (SSH Port 2022, Local SOCKS 8080)
./pax --host 1.2.3.4 --ssh-port 2022 --local-port 8080 -k "~/.ssh/key.pem"
```

## CLI Arguments Reference

| Flag | Env Var | Description |
| :--- | :--- | :--- |
| `--api` | `PAX_API_URL` | Remote API URL (Default: `https://example.com/api/auth.json`). |
| `--host` | - | Remote Server IP/Host (Triggers CLI Mode). |
| `--user` | - | Remote SSH User (Default: `root` in CLI Mode). |
| `--ssh-port` | - | Remote SSH Port (Default: `22`). |
| `--password` | - | SSH Password. |
| `-k`, `--private-key`| - | Path to local private key. |
| `-l`, `--local-port` | - | Local SOCKS5 Port (Default: `1080`). |
| `--timeout` | - | Connection/Request timeout in seconds. |

## API Response Format

Pax expects the remote URL to return a RESTful JSON wrapper containing a `data` array. If multiple nodes are returned, **Pax will safely select the first available node from the list**.

### Mode A: Password Authentication
```json
{
  "msg": "Success",
  "count": 1,
  "data": [
    {
      "auth_type": "password",
      "host": "1.1.1.1",
      "port": "22",
      "user": "root",
      "password": "my_secret_password",
      "region": "JP",
      "ref": "https://abc.com/source-page",
      "exp_at": "2026-01-16 02:45:03"
    }
  ]
}
```

### Mode B: Private Key Authentication
The `private_key` field inside the node object supports **Raw Key Content** (PEM format) OR a **File Path**.

```json
{
  "msg": "Success",
  "count": 1,
  "data": [
    {
      "auth_type": "key",
      "host": "1.1.1.2",
      "port": "22",
      "user": "root",
      
      // Option 1: Raw content of the private key
      "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\n...",
      // Option 2: Absolute path or path with ~
      // "private_key": "~/.ssh/id_rsa",

      // Optional: Passphrase if the key is encrypted
      "password": "key_passphrase",

      "region": "US",
      "ref": "https://abc.com/server-list",
      "exp_at": "2026-01-23 02:46:09"
    }
  ]
}
```

### Field Descriptions (Inside `data` objects)
| Field | Type | Description |
| :--- | :--- | :--- |
| `auth_type` | `string` | `password` or `key`. |
| `host` | `string` | Server IP or Hostname. |
| `region` | `string` | (Optional) Country code or region (e.g., "JP", "US"). |
| `ref` | `string` | (Optional) Source reference URL for display. |
| `private_key`| `string` | (Optional) Raw key content or local file path. |
| `password` | `string` | (Optional) SSH Password or Key Passphrase. |
| `exp_at` | `string` | (Optional) Expiration date (RFC3339 or "Y-m-d H:M:S"). |

## Requirements
*   **OS**: Linux, macOS, or Windows (with OpenSSH Client installed).
*   **Runtime**: The `ssh` command must be available in your `$PATH`.

## License

MIT
