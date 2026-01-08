# Pax - Automated SSH SOCKS5 Proxy

Pax is a lightweight Rust tool designed to **replace manual SSH SOCKS5 commands** (e.g., `ssh -D 1080 ...`). It can fetch credentials dynamically from a remote API **OR** take them directly from the command line, establishing the tunnel and keeping it alive automatically.

## Why use Pax?

| **Feature** | **Pax** |
| :--- | :--- |
| **Stability** | Detects disconnects (Timeout/EOF) and **auto-reconnects**. |
| **Dynamic** | Fetches credentials from a JSON API (Auto-rotate). |
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

Pax expects the remote URL to return a single JSON object.

### Mode A: Password Authentication
```json
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
```

### Mode B: Private Key Authentication
The `private_key` field supports **Raw Key Content** (PEM format) OR a **File Path**.

```json
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
```

### Field Descriptions
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
