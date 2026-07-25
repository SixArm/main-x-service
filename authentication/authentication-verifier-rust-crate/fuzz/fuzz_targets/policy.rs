//! SEC-I2 fuzz target: the ABAC policy JSON parser + rule evaluator.
//!
//! `Policy::from_json` parses a **deployment-supplied policy config**, and
//! the evaluator then runs attacker-influenced attribute matching (negation,
//! `$sub`/`$email` templates, the `resource.`/`env.` namespaces). This feeds
//! arbitrary UTF-8 to the parser and, on a parse success, evaluates the
//! policy against a fixed subject/resource/environment for every action —
//! pinning that neither the parser nor the evaluator panics on hostile
//! input.

#![no_main]

use authentication_verifier::{Action, Claims, Policy};
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn claims() -> &'static Claims {
    static C: OnceLock<Claims> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_value(serde_json::json!({
            "sub": "11111111-1111-1111-1111-111111111111",
            "email": "user@example.com",
            "name": "Test User",
            "iss": "authentication-service",
            "aud": "main-x-service",
            "exp": 9_999_999_999i64,
            "iat": 0i64,
            "sid": "sid-1",
            "attrs": { "access": ["write"], "dept": ["cardiology"], "svc": ["false"] }
        }))
        .expect("fixed claims deserialize")
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    // Fuzz the policy parser; a malformed policy is a handled `Err`.
    let Ok(policy) = Policy::from_json(s) else {
        return;
    };

    // On a parse success, exercise the evaluator across every action with a
    // fixed resource/env so the `resource.`/`env.` namespaces and the
    // `$sub`/`$email` ownership templates are reachable (owner == the
    // subject's pid). Every path must return a `Decision`, never panic.
    let resource = BTreeMap::from([
        (
            "owner".to_string(),
            vec!["11111111-1111-1111-1111-111111111111".to_string()],
        ),
        ("status".to_string(), vec!["closed".to_string()]),
    ]);
    let env = BTreeMap::from([("after_hours".to_string(), vec!["true".to_string()])]);

    for action in [
        Action::Read,
        Action::Write,
        Action::Delete,
        Action::Destructive,
    ] {
        let _ = policy.evaluate_with_context(claims(), action, "person", &resource, &env);
    }
});
