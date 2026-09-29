//! A local OIDC provider for **tests only**: the `test-idp` feature.
//!
//! [`TestIdp::start`] serves a Keycloak-shaped discovery document and JWKS
//! on an ephemeral loopback port, and [`TestIdp::token`] mints RS256 access
//! tokens the matching [`KeycloakVerifier`](crate::keycloak::KeycloakVerifier)
//! accepts. It lets a service prove its guard end to end (a Keycloak token
//! through the real ABAC decision) without a Keycloak container.
//!
//! **Never enable this feature in a production build.** The signing key is
//! published in this source, so anyone could mint a token this `IdP`'s
//! verifier accepts. A verifier only trusts the JWKS of the realm it was
//! configured with, so the feature is inert unless something is pointed at
//! a [`TestIdp`], but there is no reason to ship it.

use std::sync::atomic::{AtomicUsize, Ordering};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Realm the test `IdP` serves.
pub const REALM: &str = "mxi";
/// Audience the default test tokens carry.
pub const AUDIENCE: &str = "mxi-api";

/// A throwaway RSA key generated for tests; it protects nothing.
const PRIVATE_KEY_PEM: &str = r"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDWrWcOKD7efXrz
CiNYYPr+I9jcpEfzGh2IRNEYZg5FjUJxJjgoCoTnzF5ldeR3/bcr9bIX24+A+1fg
Lk1qaM9LxSdoJiT79yTT4YiWwDMS90JcEDo5FXRLJa2HHxoHChpzQowEmvSPw/S1
T8YHf89D9uZ40TbNvBGrIEdBh+DNFYVjFa/JWUqCoiEB2RPnm3owFvlY84N0PS1J
7Jx17tgFAHU4/RgySOzqc9cMJs5qoF/mUyuQt749bygcu4R2pHVK2A6qnb0aH18z
Vp7j3bG85J05RDqHTSl3xcZtwkzBSoXdFQv4F4yN5QBYebtHMQE4ueflwGbQL4lQ
P2JaxR/1AgMBAAECggEABM1YMs7fqSZxa6JcbAuvUaQHo9fg7CU3Z+byLnOJ+jBQ
vis2sl6Z3n2J1wcuFykLweX7F+GHckEtFAy1Gp5BlNZSLVg9F43NuxeecJDYE6KI
T3rlcoVyVoP/P7iIYoPtV4qzix7UPasKfEvOiDhpsIGDYkx7x1pfos5UCbk6ZBv+
pKkOjjTaFnu0dhFhA6svkiXncslktATti5xqm8GjwowdNdrjuXUmf+qcUrKRFSUN
wghgUo/DbkMpu0RGt1TqyaLIb4hq3Fig5ODbd8qKxoacYCDPOPESMgrZGrzlC55Y
kkgcTm8U4spBUoJGtSwjZ6VrLzxBxLRQK+15aw11AQKBgQDtk5YhXZiONOKy18wJ
D7YT+lLt/iks07Vi5L64FKP9BqH8/+SaXJKyrwz8OaXvlFILSZb2UJNR2L65Jolo
/vQNZeLVwQ7wVRouAO3V6MJbrlX44RZo5jj60h+nwhOjbqM3b35zotr1cfxqf3kn
rt7y3izUg2I5/QVrV0cnefoWoQKBgQDnUze/ccspYr0rf0u342LppOs7FMsgPyd5
RnO0J0YM51ctfS6RVQkB0Qk2y7ZYE8AHPQRwzvhrV1BkD1cheSZubvyLR7TMouIM
9ghU6rC3T5/ycoPZAcUNn4eI6jK3NAaeS/DysYujtFTVRxYEf7musmsNrpoSHTDy
+QgPeqjM1QKBgD62uubL4j2H0GANfxrVPuc/KS0R7qSarkMQxxeouzFL0u8KTbpB
vafVdcQPI7J+oLnCD0uuMXVlldMiTRueUaZHwv1SHTNsA6EpNZ9F9ihleORd5qCC
RYBQf3K1VKHLzuIPWH4F+27XTB6Az+adztSluYfPtto/5HJVc78D//SBAoGBALoa
TC5cKTtpcZFt/we5Cxm3kXvdtbGCvYom8O0N76Bv+cXiATXw+KcaelQ4PcHMWeA4
6bqr+FW7UDS/1rRaWF3eMpUHImDD5iLRSVCv424GxEJ0eLh8YQEdyBeRey3C0FRH
+lf2GMaiTOGtJ/yEmWj/p3rBbriF/ZxsV1zErMrlAoGBAJdg99gYJ3sCe0kUA5XD
cKPX7V1THWXUPLU4Z30MFCZpORyykMhWJyq8qDAvLgdWqA4xHB/APKnKlLTElbxU
YKTjfrCO2LTfklSgJKEHhI1ZC7ln6v2/TR83YzXhSUo+dgZuSO947rC8NLZSuxOh
zwttb6r2wIt4FGRgKrCtJaqe
-----END PRIVATE KEY-----";
/// The matching public modulus, base64url.
const JWK_N: &str = "1q1nDig-3n168wojWGD6_iPY3KRH8xodiETRGGYORY1CcSY4KAqE58xeZXXkd_23K_WyF9uPgPtX4C5NamjPS8UnaCYk-_ck0-GIlsAzEvdCXBA6ORV0SyWthx8aBwoac0KMBJr0j8P0tU_GB3_PQ_bmeNE2zbwRqyBHQYfgzRWFYxWvyVlKgqIhAdkT55t6MBb5WPODdD0tSeycde7YBQB1OP0YMkjs6nPXDCbOaqBf5lMrkLe-PW8oHLuEdqR1StgOqp29Gh9fM1ae492xvOSdOUQ6h00pd8XGbcJMwUqF3RUL-BeMjeUAWHm7RzEBOLnn5cBm0C-JUD9iWsUf9Q";

static JTI: AtomicUsize = AtomicUsize::new(0);

/// A running local OIDC provider.
#[derive(Debug, Clone)]
pub struct TestIdp {
    /// Base URL to give `KeycloakSettings::url`, e.g. `http://127.0.0.1:41234`.
    pub base_url: String,
}

fn now() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(i64::MAX)
}

impl TestIdp {
    /// Start the provider on an ephemeral loopback port. It serves until
    /// the Tokio runtime shuts down.
    ///
    /// # Panics
    ///
    /// If it cannot bind a loopback port (a test environment failure).
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let base = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("local addr").port()
        );
        let discovery = json!({
            "issuer": format!("{base}/realms/{REALM}"),
            "authorization_endpoint": format!("{base}/auth"),
            "token_endpoint": format!("{base}/token"),
            "jwks_uri": format!("{base}/certs"),
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
        })
        .to_string();
        let jwks = json!({ "keys": [{
            "kty": "RSA", "alg": "RS256", "use": "sig", "kid": "k1", "n": JWK_N, "e": "AQAB"
        }]})
        .to_string();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let (discovery, jwks) = (discovery.clone(), jwks.clone());
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let head = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let (status, body) = if path.ends_with("/openid-configuration") {
                        ("200 OK", discovery)
                    } else if path == "/certs" {
                        ("200 OK", jwks)
                    } else {
                        ("404 Not Found", "{}".to_string())
                    };
                    let resp = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        Self { base_url: base }
    }

    /// The issuer this `IdP`'s tokens carry: `{base}/realms/mxi`.
    #[must_use]
    pub fn issuer(&self) -> String {
        format!("{}/realms/{REALM}", self.base_url)
    }

    /// Mint an RS256 access token for a verified user with realm roles
    /// `editor` and `offline_access`, valid for five minutes. `tweak` edits
    /// the claim object before signing (e.g. change `aud`, `exp`, or
    /// `realm_access.roles`).
    ///
    /// # Panics
    ///
    /// If signing fails (the embedded key is valid, so it does not).
    pub fn token(&self, tweak: impl FnOnce(&mut Value)) -> String {
        let mut claims = json!({
            "iss": self.issuer(),
            "aud": [AUDIENCE],
            "sub": "0c4f1e2a-0000-4000-8000-000000000001",
            "azp": "mxi-web",
            "typ": "Bearer",
            "jti": format!("jti-{}", JTI.fetch_add(1, Ordering::Relaxed)),
            "iat": now() - 10,
            "exp": now() + 300,
            "email": "ada@example.com",
            "email_verified": true,
            "name": "Ada",
            "preferred_username": "ada",
            "sid": "sess-1",
            "realm_access": { "roles": ["editor", "offline_access"] },
        });
        tweak(&mut claims);
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("k1".into());
        jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM.as_bytes()).expect("embedded test key"),
        )
        .expect("sign test token")
    }
}
