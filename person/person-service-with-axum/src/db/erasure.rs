//! The statements of a GDPR Art. 17 erasure
//! ([`crate::compliance::erasure`], which orders them and says why).
//!
//! Each runs on the caller's connection, which the erase handler makes one
//! transaction, so the erasure commits or rolls back whole.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::Result;

/// Whether `persons` holds `id`, and if so whether it is active.
///
/// # Errors
///
/// When the query fails.
pub async fn person_activity(conn: &mut PgConnection, id: Uuid) -> Result<Option<bool>> {
    let active = sqlx::query_scalar!("SELECT active FROM persons WHERE id = $1", id)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(active)
}

/// Delete every row of one person-owned child `table` (each keyed by
/// `person_id`), returning how many went. `table` is one of
/// [`crate::compliance::erasure`]'s child tables; each has its own literal
/// statement, so a table name never reaches SQL as text.
///
/// # Errors
///
/// When the delete fails, or `table` is not a person-owned child table.
pub async fn delete_child_rows(conn: &mut PgConnection, table: &str, id: Uuid) -> Result<u64> {
    let done = match table {
        "person_names" => {
            sqlx::query!("DELETE FROM person_names WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_identifiers" => {
            sqlx::query!("DELETE FROM person_identifiers WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_addresses" => {
            sqlx::query!("DELETE FROM person_addresses WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_contacts" => {
            sqlx::query!("DELETE FROM person_contacts WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_documents" => {
            sqlx::query!("DELETE FROM person_documents WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_photos" => {
            sqlx::query!("DELETE FROM person_photos WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_links" => {
            sqlx::query!("DELETE FROM person_links WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        "person_match_scores" => {
            sqlx::query!("DELETE FROM person_match_scores WHERE person_id = $1", id)
                .execute(&mut *conn)
                .await?
        }
        other => {
            return Err(crate::Error::Internal(format!(
                "{other} is not a person-owned child table"
            )));
        }
    };
    Ok(done.rows_affected())
}

/// Delete a person's emergency contacts and, first, their telecom rows,
/// which hang off the contacts rather than the person. Returns
/// `(telecom rows, contact rows)` deleted.
///
/// # Errors
///
/// When a delete fails.
pub async fn delete_emergency_contacts(conn: &mut PgConnection, id: Uuid) -> Result<(u64, u64)> {
    let telecom = sqlx::query!(
        "DELETE FROM person_emergency_contact_telecom WHERE emergency_contact_id IN
             (SELECT id FROM person_emergency_contacts WHERE person_id = $1)",
        id,
    )
    .execute(&mut *conn)
    .await?;
    let contacts = sqlx::query!(
        "DELETE FROM person_emergency_contacts WHERE person_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok((telecom.rows_affected(), contacts.rows_affected()))
}

/// Scrub the parent row's own personal fields and retire it: `gender`,
/// which is `NOT NULL`, takes `gender`; the two digests become `NULL`.
///
/// # Errors
///
/// When the update fails.
pub async fn scrub_person(
    conn: &mut PgConnection,
    id: Uuid,
    gender: &str,
    actor: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE persons SET
           gender = $2, birth_date = NULL, tax_id = NULL,
           deceased = FALSE, deceased_datetime = NULL,
           marital_status = NULL, multiple_birth = NULL,
           managing_organization_id = NULL,
           created_by = NULL, updated_by = NULL,
           content_hash = NULL, content_hash_sha3 = NULL,
           active = FALSE, deleted_at = NOW(), deleted_by = $3,
           updated_at = NOW()
         WHERE id = $1",
        id,
        gender,
        actor,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Write back one tombstone name, `family` with no given names, prefixes
/// or suffixes.
///
/// # Errors
///
/// When the insert fails.
pub async fn insert_tombstone_name(conn: &mut PgConnection, id: Uuid, family: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO person_names (id, person_id, use_type, family, given, prefix, suffix)
         VALUES ($1, $2, NULL, $3, ARRAY[]::text[], ARRAY[]::text[], ARRAY[]::text[])",
        Uuid::new_v4(),
        id,
        family,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Withdraw every live cross-service link from `id`, returning how many.
///
/// # Errors
///
/// When the update fails.
pub async fn withdraw_links(conn: &mut PgConnection, id: Uuid) -> Result<u64> {
    let done = sqlx::query!(
        "UPDATE entity_links SET deleted_at = NOW()
         WHERE from_pid = $1 AND deleted_at IS NULL",
        id,
    )
    .execute(&mut *conn)
    .await?;
    Ok(done.rows_affected())
}
