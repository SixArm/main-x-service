//! Whether this deployment accepts magic-link sign-in.
//!
//! A deployment federated to an enterprise identity provider (NHS Wales
//! Entra ID, for one) may have to accept no local login at all
//! (`spec/nhs-wales-supplier-assurance` NWS-A3). With
//! [`ENABLED_ENV`] set to `false`, the two routes that issue a magic
//! link by email, `POST /api/auth/signup` and `POST /api/auth/magic-link`,
//! are not mounted, so they answer `404` like any other absent route.
//!
//! `GET /api/auth/magic-link/{token}` stays mounted. The OIDC callback
//! hands its verified user to the front end through a single-use token
//! redeemed there (`agents/share/authentication-sessions.md` §7a), and with
//! email issuance gone the identity provider is the only thing that can mint one.
//!
//! Turning magic links off without a working identity provider would lock everyone
//! out, so [`ensure_sign_in_path`] refuses to start the server then.

/// `true` (the default when unset) or `false`. Anything else stops the
/// server at boot, so a typo cannot leave local login on unnoticed.
pub const ENABLED_ENV: &str = "AUTH_MAGIC_LINK_ENABLED";

/// Read [`ENABLED_ENV`].
///
/// # Errors
///
/// When the value is set to anything but `true`, `false`, `1` or `0`
/// (any case, surrounding space ignored).
pub fn enabled_from_env() -> Result<bool, String> {
    parse(std::env::var(ENABLED_ENV).ok().as_deref())
}

/// Parse a value of [`ENABLED_ENV`]; unset or empty means enabled.
///
/// # Errors
///
/// When the value is not one of the accepted spellings.
pub fn parse(value: Option<&str>) -> Result<bool, String> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(true);
    };
    if value == "1" || value.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if value == "0" || value.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(format!(
            "{ENABLED_ENV} must be `true` or `false`, not {value:?}"
        ))
    }
}

/// Refuse a configuration with no way to sign in.
///
/// # Errors
///
/// When magic links are off and no identity provider is configured.
pub fn ensure_sign_in_path(magic_link_enabled: bool, idp_configured: bool) -> Result<(), String> {
    if magic_link_enabled || idp_configured {
        Ok(())
    } else {
        Err(format!(
            "{ENABLED_ENV}=false leaves no way to sign in: configure OIDC \
             (the `oidc` build feature and AUTH_OIDC_ISSUER_URL, \
             AUTH_OIDC_CLIENT_ID, the client secret and AUTH_OIDC_REDIRECT_URL) \
             or leave magic links on"
        ))
    }
}

/// Whether OIDC federation is both compiled in and fully configured.
#[must_use]
pub fn idp_configured() -> bool {
    #[cfg(feature = "oidc")]
    {
        crate::oidc::OidcConfig::from_env().is_some()
    }
    #[cfg(not(feature = "oidc"))]
    {
        false
    }
}

/// Check both settings together, for the server's boot.
///
/// # Errors
///
/// When [`ENABLED_ENV`] is malformed, or magic links are off with no identity provider.
pub fn ensure_from_env() -> Result<bool, String> {
    let enabled = enabled_from_env()?;
    ensure_sign_in_path(enabled, idp_configured())?;
    Ok(enabled)
}

#[cfg(test)]
mod tests {
    use super::{ensure_sign_in_path, parse};

    #[test]
    fn unset_or_empty_means_enabled() {
        assert_eq!(parse(None), Ok(true));
        assert_eq!(parse(Some("")), Ok(true));
        assert_eq!(parse(Some("  ")), Ok(true));
    }

    #[test]
    fn accepted_spellings() {
        for on in ["true", "TRUE", "1", " true "] {
            assert_eq!(parse(Some(on)), Ok(true), "{on:?}");
        }
        for off in ["false", "False", "0", " false "] {
            assert_eq!(parse(Some(off)), Ok(false), "{off:?}");
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in ["flase", "off", "no", "yes", "2"] {
            assert!(parse(Some(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn some_way_to_sign_in_is_required() {
        assert!(ensure_sign_in_path(true, false).is_ok());
        assert!(ensure_sign_in_path(true, true).is_ok());
        assert!(ensure_sign_in_path(false, true).is_ok());
        assert!(ensure_sign_in_path(false, false).is_err());
    }
}
