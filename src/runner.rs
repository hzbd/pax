use crate::config::{SshConfig, AuthType};
use anyhow::{anyhow, Result};
use expectrl::{Eof, Regex, Session};
use std::process::Command;
use tracing::{info, warn, debug};

/// Starts the SSH process using `expectrl`.
pub fn start_ssh_process(local_port: u16, config: &SshConfig) -> Result<()> {
    let mut cmd = Command::new("ssh");

    cmd.arg("-D").arg(local_port.to_string())
       .arg("-N") // No remote command (forwarding only)
       .arg("-C") // Compression
       .arg("-v") // Verbose (helps debugging, but we rely on process state)
       .arg("-o").arg("StrictHostKeyChecking=no")
       .arg("-o").arg("UserKnownHostsFile=/dev/null")
       .arg("-o").arg("ServerAliveInterval=15")
       .arg("-o").arg("ConnectTimeout=10");

    if config.auth_type == AuthType::Key {
        if let Some(ref key_path) = config.private_key {
            debug!("Using private key path: {}", key_path);
            cmd.arg("-i").arg(key_path);
        }
    }

    cmd.arg("-p").arg(&config.port)
       .arg(format!("{}@{}", config.user, config.host));

    info!("Executing SSH process...");

    let mut p = Session::spawn(cmd).map_err(|e| anyhow!("Failed to spawn SSH: {}", e))?;

    // --- INTERACTION PHASE ---
    // We give SSH a few seconds to prompt for password or fail.
    // If it says nothing for 5 seconds but stays alive, we assume success.
    p.set_expect_timeout(Some(std::time::Duration::from_secs(5)));

    loop {
        // Watch for specific prompts or errors
        let result = p.expect(Regex("password:|Enter passphrase|Connection refused|timed out|denied"));

        match result {
            Ok(output) => {
                let match_str = String::from_utf8_lossy(output.get(0).unwrap_or(&[]));
                let buf_str = String::from_utf8_lossy(output.before());

                // 1. Password Prompt
                if match_str.contains("password:") {
                    if config.auth_type == AuthType::Password {
                        if let Some(ref pwd) = config.password {
                            info!("Sending password...");
                            p.send_line(pwd)?;
                            continue; // Continue loop to check if accepted
                        } else {
                            return Err(anyhow!("Server asked for password but none provided!"));
                        }
                    } else {
                        return Err(anyhow!("Server asked for password, but AuthType is Key."));
                    }
                }

                // 2. Key Passphrase Prompt
                if match_str.contains("Enter passphrase") {
                    info!("Key passphrase required.");
                    if let Some(ref pwd) = config.password {
                        info!("Sending passphrase...");
                        p.send_line(pwd)?;
                        continue;
                    } else {
                         return Err(anyhow!("Passphrase required but 'password' field is empty!"));
                    }
                }

                // 3. Explicit Errors
                if buf_str.contains("Connection refused") || buf_str.contains("timed out") {
                    return Err(anyhow!("Connection failed (Refused/Timeout)"));
                }
                if buf_str.contains("denied") {
                    return Err(anyhow!("Permission denied (Wrong password/key?)"));
                }
            },
            Err(expectrl::Error::ExpectTimeout) => {
                // --- SUCCESS CHECK ---
                // The expect timed out. This means SSH is silent.
                // If the process is still running, it means the connection is likely established.
                if is_process_alive(&mut p) {
                    info!("Tunnel established (Silent Mode). SOCKS5: 127.0.0.1:{}", local_port);
                    break; // Exit the interaction loop, move to monitoring
                } else {
                    return Err(anyhow!("SSH process died unexpectedly during initialization."));
                }
            },
            Err(e) => {
                return Err(anyhow!("Interaction error: {}", e));
            }
        }
    }

    // --- MONITORING PHASE ---
    // Disable timeout, just wait for the process to exit (e.g., network drop)
    p.set_expect_timeout(None);

    match p.expect(Eof) {
        Ok(_) => {
            warn!("SSH process exited (EOF).");
            Err(anyhow!("SSH exited normally"))
        }
        Err(e) => Err(anyhow!("Monitor error: {}", e)),
    }
}

/// Helper: Checks if the spawned process is still running
/// Handles platform differences in expectrl's API.
#[cfg(unix)]
fn is_process_alive(p: &mut Session) -> bool {
    // Unix: returns Result<bool>
    p.get_process_mut().is_alive().unwrap_or(false)
}

#[cfg(windows)]
fn is_process_alive(p: &mut Session) -> bool {
    // Windows: returns bool
    p.get_process_mut().is_alive()
}
