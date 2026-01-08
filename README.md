# Pax - Automated SSH SOCKS5 Proxy

Pax is a lightweight Rust tool designed to **replace manual SSH SOCKS5 commands** (e.g., `ssh -D 1080 ...`). Instead of hardcoding credentials, Pax fetches them dynamically from a remote API, establishes the tunnel, and keeps it alive.

## Why use Pax?

| **Manual Method** | **Pax** |
| :--- | :--- |
| You run `ssh -D 1080 -N -C user@host` | Pax runs this automatically in the background. |
| Credentials are static or typed manually | Credentials are fetched from a JSON API (Auto-rotate). |
| Connection drops? You must restart it | **Auto-reconnects** immediately upon failure. |
| Complex private key management | Handles **Local Keys** (`~/.ssh/...`) & **API Keys** (Raw content). |

## Features

*   **Dynamic Config**: Fetches Host, User, Port, Password/Key from a URL.
*   **Environment Friendly**: Supports configuration via **Environment Variables** (`PAX_API_URL`).
*   **Silent Mode Support**: Compatible with SSH servers that suppress output (`-N` mode), automatically detecting successful connections.
*   **Expiration Aware**: Visual alerts if the account is expiring soon (<24h).
*   **Smart Paths**: Automatically expands `~` to your home directory for private key paths.
*   **Metadata Display**: Shows server **Region** and **Source Ref** for better tracking.

![State](./screenshot.png)
**Display under normal conditions.**

![State with datetime exp notify](./screenshot_notify.png)
**A prominent reminder appears on startup when authentication is about to expire.**

## Usage

### 1. Build & Run
```bash
cargo build --release
./target/release/pax
```

### 2. Configuration & Arguments

Pax prioritizes configuration in this order: **CLI Flags > Environment Variables > Default Values**.

#### Via Environment Variable (Recommended for Docker/Scripts)
```bash
export PAX_API_URL="https://my-api.com/get-ssh-config"
./pax
```

#### Via Command Line Flags
```bash
# Default behavior (uses default API or Env Var)
./pax

# Override API endpoint manually
./pax --api "http://127.0.0.1:8000/config.json"

# Force use of a local private key (Overrides API auth)
# Pax supports tilde expansion (e.g., ~/.ssh/id_rsa)
./pax -k "~/.ssh/id_rsa"

# Change local SOCKS5 port (Default: 1080)
./pax --local-port 2080
```

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
