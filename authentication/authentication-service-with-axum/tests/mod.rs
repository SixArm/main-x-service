//! Integration-test entry point.
//!
//! Aggregates the test submodules into one test binary. The model and
//! HTTP-request suites boot the app and so require PostgreSQL —
//! those tests are `#[ignore]`d to keep a DB-free `cargo test` green
//! (run them with `cargo test -- --ignored`). The DB-free contract /
//! shape assertions inside `requests` run by default.

mod models;
mod requests;
mod tasks;
mod workers;

/// Insert the two fixture users the model tests look up.
pub async fn seed_users(ctx: &axa::AppContext) -> Result<(), sqlx::Error> {
    let password = "$argon2id$v=19$m=19456,t=2,p=1$ETQBx4rTgNAZhSaeYZKOZg$eYTdH26CRT6nUJtacLDEboP0li6xUwUF/q5nSlQ8uuc";
    let fixtures = [
        (
            "11111111-1111-1111-1111-111111111111",
            "user1@example.com",
            "lo-95ec80d7-cb60-4b70-9b4b-9ef74cb88758",
            "user1",
        ),
        (
            "22222222-2222-2222-2222-222222222222",
            "user2@example.com",
            "lo-153561ca-fa84-4e1b-813a-c62526d0a77e",
            "user2",
        ),
    ];
    let created = chrono::DateTime::parse_from_rfc3339("2023-11-12T12:34:56.789Z")
        .expect("fixture timestamp");
    for (id, (pid, email, api_key, name)) in (1_i64..).zip(fixtures) {
        // A plain INSERT, not `users::Model::insert`, which would replace
        // these fixed `pid`s with random ones.
        sqlx::query!(
            "INSERT INTO users
                 (id, pid, email, password, api_key, name, attributes, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            id,
            pid.parse::<uuid::Uuid>().expect("fixture pid"),
            email,
            password,
            api_key,
            name,
            serde_json::json!({}),
            created,
            created,
        )
        .execute(&ctx.db)
        .await?;
    }
    Ok(())
}

/// Snapshot filters that hide the values that change on every run.
pub fn cleanup_user_model() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
            "PID",
        ),
        (r#"password: "[^"]*""#, "password: \"PASSWORD\""),
        (r"([A-Za-z0-9-_]*\.[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*)", "TOKEN"),
        (
            r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?[+-]\d{2}:\d{2}",
            "DATE",
        ),
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+", "DATE"),
        (r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})", "DATE"),
        (r"id: \d+,", "id: ID"),
    ]
}
