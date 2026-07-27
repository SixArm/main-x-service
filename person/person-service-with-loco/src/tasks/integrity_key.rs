//! `integrity_key` task — generate and inspect the **data-protection
//! key** that backs the integrity MACs.
//!
//! The MAC key ([`crate::compliance::mac`]) is the one secret this
//! service holds that the database must never see. Generating it by hand
//! is where that goes wrong: `openssl rand -hex 32` is fine, but the
//! failure modes people actually hit — a short key, a placeholder that
//! got committed, a key pasted with a stray newline or an `0x` prefix —
//! all produce something that *looks* like a key and silently disables
//! or weakens MACs. This task makes the correct thing the easy thing,
//! and gives an operator a way to check a candidate before deploying it.
//!
//! ```text
//! # generate a fresh 32-byte key (hex on stdout, nothing else)
//! cargo loco task integrity_key
//! cargo loco task integrity_key op:generate
//!
//! # write it straight to a file for a secret mount, mode 0600
//! cargo loco task integrity_key op:generate out:/run/secrets/mac.key
//!
//! # check a candidate key without deploying it
//! cargo loco task integrity_key op:check key:9f86d081884c7d65…
//!
//! # report what this process currently has loaded (never the key itself)
//! cargo loco task integrity_key op:status
//! ```
//!
//! ## Why the key is printed and never logged
//!
//! `op:generate` writes the key to **stdout only**. It is never passed to
//! `tracing`, because a log line is exactly the wrong place for it — logs
//! are shipped off-box, retained, and indexed, which is the opposite of
//! what this key needs (the no-secret-in-logs invariant,
//! `agents/share/security.md` §3.9). For the same reason `op:status`
//! reports the key *id* and whether a key loaded, never any key material.
//!
//! The generated key is 32 bytes because that matches HMAC-SHA256's block
//! security and [`mac::MIN_KEY_LEN`]. Longer buys nothing here: HMAC
//! hashes a key longer than its block size back down.

use loco_rs::prelude::*;

use crate::compliance::mac;

/// What the operator asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCommand {
    /// Generate a fresh key; `out` writes it to a file instead of stdout.
    Generate {
        /// Destination path, or `None` for stdout.
        out: Option<String>,
    },
    /// Check a candidate key without deploying it.
    Check {
        /// The candidate, hex-encoded.
        key: String,
    },
    /// Report what this process has loaded.
    Status,
}

/// Parse the `key:value` argument style loco tasks use.
///
/// Defaults to [`KeyCommand::Generate`] with no output path, because
/// generating is the overwhelmingly common reason to reach for this.
///
/// # Errors
///
/// When `op:` names something other than generate / check / status, or
/// `op:check` is given without a `key:`.
pub fn parse(vars: &std::collections::BTreeMap<String, String>) -> Result<KeyCommand, String> {
    match vars.get("op").map_or("generate", String::as_str) {
        "generate" | "gen" | "new" => Ok(KeyCommand::Generate {
            out: vars.get("out").cloned(),
        }),
        "check" | "verify" => vars.get("key").map_or_else(
            || Err("op:check needs key:<hex>".to_string()),
            |key| {
                Ok(KeyCommand::Check {
                    key: key.trim().to_string(),
                })
            },
        ),
        "status" => Ok(KeyCommand::Status),
        other => Err(format!(
            "unknown op:{other} — expected generate, check, or status"
        )),
    }
}

/// Assess a candidate key, and generate/write one.
///
/// All re-exported from the shared `integrity-mac` crate rather than
/// reimplemented: a checker that disagreed with the loader would be
/// worse than no checker, because it would bless a key the service then
/// refuses.
use crate::compliance::mac::{assess_key_hex as assess, generate_key, write_key_file};

/// The `integrity_key` CLI task.
pub struct IntegrityKey;

#[async_trait]
impl Task for IntegrityKey {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "integrity_key".to_string(),
            detail: "Generate, check, or report the integrity MAC key".to_string(),
        }
    }

    async fn run(&self, _ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let command = parse(&vars.cli).map_err(|e| Error::string(&e))?;
        match command {
            KeyCommand::Generate { out } => {
                let key = generate_key().map_err(|e| Error::string(&e))?;
                match out {
                    None => {
                        // stdout only — never `tracing`.
                        println!("{key}");
                    }
                    Some(path) => {
                        write_key_file(&path, &key).map_err(|e| Error::string(&e))?;
                        println!(
                            "wrote a new {}-byte key to {path} (mode 0600)",
                            integrity_mac::KEY_BYTES
                        );
                        println!("point {} at it", mac::KEY_FILE_ENV);
                    }
                }
            }
            KeyCommand::Check { key } => {
                let assessment = assess(&key);
                println!("{}", assessment.describe());
                if !assessment.is_usable() {
                    return Err(Error::string("candidate key is not usable"));
                }
            }
            KeyCommand::Status => {
                if mac::is_enabled() {
                    println!(
                        "integrity MACs are ENABLED (active key id: {})",
                        mac::active_key_id().unwrap_or("?")
                    );
                } else {
                    println!(
                        "integrity MACs are DISABLED — set {} or {}",
                        mac::KEY_FILE_ENV,
                        mac::KEY_ENV
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyCommand, parse};

    // The key rules themselves — generation, assessment, file
    // permissions, clobber refusal — are tested in the shared
    // `integrity-mac` crate, which owns them. What is local here is only
    // how the command line maps onto them.

    fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// Generating is the default, since it is why anyone runs this.
    #[test]
    fn no_arguments_means_generate() {
        assert_eq!(
            parse(&vars(&[])).expect("parses"),
            KeyCommand::Generate { out: None }
        );
    }

    /// The operations parse, including their aliases.
    #[test]
    fn operations_parse() {
        assert_eq!(
            parse(&vars(&[("op", "generate"), ("out", "/tmp/k")])).expect("parses"),
            KeyCommand::Generate {
                out: Some("/tmp/k".to_string())
            }
        );
        assert_eq!(
            parse(&vars(&[("op", "check"), ("key", " ab ")])).expect("parses"),
            KeyCommand::Check {
                key: "ab".to_string()
            }
        );
        assert_eq!(
            parse(&vars(&[("op", "status")])).expect("parses"),
            KeyCommand::Status
        );
        assert!(parse(&vars(&[("op", "nonsense")])).is_err());
        assert!(
            parse(&vars(&[("op", "check")])).is_err(),
            "check without a key must not silently check nothing"
        );
    }
}
