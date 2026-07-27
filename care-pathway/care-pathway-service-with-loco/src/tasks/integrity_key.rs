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

/// Bytes in a generated key.
const KEY_BYTES: usize = 32;

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

/// Why a candidate key is unusable, or that it is fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assessment {
    /// Usable as an active key.
    Usable {
        /// Decoded length in bytes.
        bytes: usize,
    },
    /// Not valid hex — the commonest paste error, e.g. an `0x` prefix, a
    /// stray quote, or an odd number of characters.
    NotHex,
    /// Shorter than [`mac::MIN_KEY_LEN`].
    TooShort {
        /// Decoded length in bytes.
        bytes: usize,
    },
    /// Full length but too few distinct bytes to be a real secret.
    Placeholder {
        /// How many distinct byte values appeared.
        distinct: usize,
    },
}

/// Assess a candidate key against the same rules the loader applies.
///
/// Deliberately reuses [`mac::assess_key_hex`] rather than
/// re-implementing the checks: a checker that disagreed with the loader
/// would be worse than no checker, because it would bless a key the
/// service then refuses.
#[must_use]
pub fn assess(candidate: &str) -> Assessment {
    mac::assess_key_hex(candidate)
}

/// Human-readable verdict for [`Assessment`].
#[must_use]
pub fn describe(assessment: &Assessment) -> String {
    match assessment {
        Assessment::Usable { bytes } => {
            format!("usable: {bytes} bytes of valid hex")
        }
        Assessment::NotHex => "NOT USABLE: not valid hex (an odd length, an 0x prefix, \
             whitespace, or a stray quote are the usual causes)"
            .to_string(),
        Assessment::TooShort { bytes } => {
            format!("NOT USABLE: {bytes} bytes, minimum is {}", mac::MIN_KEY_LEN)
        }
        Assessment::Placeholder { distinct } => format!(
            "NOT USABLE: only {distinct} distinct byte values — this looks like a \
             placeholder rather than a generated key"
        ),
    }
}

/// Generate a key from the operating system CSPRNG, hex-encoded.
///
/// `getrandom` is the OS entropy source directly (`getrandom(2)`,
/// `BCryptGenRandom`) rather than a userspace PRNG. For a key that lives
/// for months that is the right call: there is no seeding question, no
/// process-fork hazard, and no PRNG state to reason about.
///
/// # Errors
///
/// When the OS entropy source is unavailable, which is fatal rather than
/// something to paper over — a fallback here would be a predictable key.
pub fn generate() -> std::result::Result<String, String> {
    use std::fmt::Write as _;

    let mut bytes = [0u8; KEY_BYTES];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| format!("the operating system entropy source failed: {e}"))?;
    let mut hex = String::with_capacity(KEY_BYTES * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }

    // A generated key that failed our own rules would mean the generator
    // is broken; checking is nearly free and the alternative is shipping
    // it. 32 random bytes have ~2^-60 odds of under 8 distinct values, so
    // this firing at all means something is very wrong.
    if !matches!(assess(&hex), Assessment::Usable { .. }) {
        return Err("generated key failed its own validation; refusing to emit it".to_string());
    }
    Ok(hex)
}

/// Write a key to `path` with owner-only permissions.
///
/// Created at mode `0600` on Unix **before** the bytes are written, not
/// after: creating world-readable and then tightening leaves a window in
/// which any local user can read the key, and that window is exactly when
/// the interesting bytes land.
///
/// # Errors
///
/// When the file cannot be created or written.
pub fn write_key_file(path: &str, key: &str) -> std::result::Result<(), String> {
    use std::io::Write as _;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    // `create_new` so an existing key is never silently overwritten:
    // clobbering the live key would make every stored MAC unverifiable,
    // and it is not recoverable.
    let mut file = options
        .open(path)
        .map_err(|e| format!("cannot create {path}: {e} (it must not already exist)"))?;
    writeln!(file, "{key}").map_err(|e| format!("cannot write {path}: {e}"))
}

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
                let key = generate().map_err(|e| Error::string(&e))?;
                match out {
                    None => {
                        // stdout only — never `tracing`.
                        println!("{key}");
                    }
                    Some(path) => {
                        write_key_file(&path, &key).map_err(|e| Error::string(&e))?;
                        println!("wrote a new {KEY_BYTES}-byte key to {path} (mode 0600)");
                        println!("point {} at it", mac::KEY_FILE_ENV);
                    }
                }
            }
            KeyCommand::Check { key } => {
                let assessment = assess(&key);
                println!("{}", describe(&assessment));
                if !matches!(assessment, Assessment::Usable { .. }) {
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
    use super::{Assessment, KeyCommand, assess, describe, generate, parse, write_key_file};

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

    /// A generated key is usable, full length, and different every time.
    ///
    /// The last part matters: a generator returning a constant would pass
    /// every other check in this file.
    #[test]
    fn generated_keys_are_usable_and_distinct() {
        let a = generate().expect("generates");
        let b = generate().expect("generates");
        assert_eq!(a.len(), 64, "32 bytes as hex");
        assert_ne!(a, b, "two calls must not return the same key");
        assert_eq!(assess(&a), Assessment::Usable { bytes: 32 });
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The checker rejects exactly what the loader rejects. A checker that
    /// blessed a key the service then refused would be worse than none.
    #[test]
    fn the_checker_agrees_with_the_loader() {
        assert_eq!(
            assess("00".repeat(32).as_str()),
            Assessment::Placeholder { distinct: 1 }
        );
        assert_eq!(assess("abcd"), Assessment::TooShort { bytes: 2 });
        assert_eq!(assess("zz".repeat(32).as_str()), Assessment::NotHex);
        assert_eq!(assess("0x00"), Assessment::NotHex, "an 0x prefix");
        assert!(matches!(
            assess(&generate().expect("generates")),
            Assessment::Usable { .. }
        ));
    }

    /// Every verdict explains itself; an operator reading "NOT USABLE"
    /// with no reason has to guess.
    #[test]
    fn every_verdict_is_explained() {
        for a in [
            Assessment::Usable { bytes: 32 },
            Assessment::NotHex,
            Assessment::TooShort { bytes: 4 },
            Assessment::Placeholder { distinct: 2 },
        ] {
            let text = describe(&a);
            assert!(text.len() > 20, "{a:?} explanation is too thin: {text}");
            if !matches!(a, Assessment::Usable { .. }) {
                assert!(text.contains("NOT USABLE"), "{a:?} must be unambiguous");
            }
        }
    }

    /// Writing refuses to clobber an existing key. Overwriting the live
    /// key makes every stored MAC unverifiable and is not recoverable.
    #[test]
    fn writing_never_overwrites_an_existing_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        let path = path.to_str().expect("utf-8");

        write_key_file(path, "first").expect("first write succeeds");
        let err = write_key_file(path, "second").expect_err("second write must fail");
        assert!(err.contains("already exist"), "{err}");
        assert!(
            std::fs::read_to_string(path)
                .expect("read")
                .contains("first"),
            "the original key must survive"
        );
    }

    /// On Unix the key file is owner-only from the moment it exists.
    #[cfg(unix)]
    #[test]
    fn the_key_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        let path_str = path.to_str().expect("utf-8");
        write_key_file(path_str, "abc").expect("write");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "key file must be mode 0600");
    }
}
