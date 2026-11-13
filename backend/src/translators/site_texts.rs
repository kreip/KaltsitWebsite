//! The site's Russian around the section (the header, the menus, the sign-in
//! dialog, the shared buttons: `site-ru.json`), loaded into the site's
//! translations (`ui_messages`, locale `ru`) at every start and from the
//! admin panel's "Refresh site texts". Only keys with no Russian yet are
//! written, so a translation made in the admin panel is never overwritten —
//! except the key that names the section itself (its menu item), written
//! every time so a rename reaches the site. A key the site's catalog does
//! not know yet (`bun run i18n:sync` not run) is skipped and counted.
//!
//! The section's own texts (`translators.*`) are not loaded: the page ships
//! them (`translators.ru.json`) and shows the catalog's value only where a
//! person translated the key in the admin panel (`SectionLocale.tsx`). Rows
//! an earlier loader wrote for them (no author) are dropped, so the shipped
//! file stays the source.

use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;

const SITE_RU: &str = include_str!("site-ru.json");

/// Keys of `site-ru.json` that name the section itself: written every time.
const OWN_KEYS: &[&str] = &["nav.header.translators"];

/// What a load did.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SiteTextsLoaded {
    /// Keys in the file.
    pub total: i32,
    /// Keys written now (new Russian, or the section's own keys).
    pub written: i32,
    /// Keys the site's catalog does not know yet (the frontend's `i18n:sync` has not run).
    pub unknown: i32,
}

fn texts() -> BTreeMap<String, String> {
    serde_json::from_str(SITE_RU).expect("site-ru.json is a JSON object of strings")
}

/// Create the `ru` locale (switched off: showing it to everyone is the site
/// owner's call, in the admin panel) and write the file's texts.
pub async fn load(pool: &PgPool) -> Result<SiteTextsLoaded, sqlx::Error> {
    sqlx::query(
        "INSERT INTO locales (code, english_name, native_name, fallback_locale, gamedata_server, enabled, sort_order)
         VALUES ('ru', 'Russian', 'Русский', 'en', 'en', false, 10)
         ON CONFLICT (code) DO NOTHING",
    )
    .execute(pool)
    .await?;
    let texts = texts();
    let keys: Vec<&str> = texts.keys().map(String::as_str).collect();
    let values: Vec<&str> = texts.values().map(String::as_str).collect();
    let own: Vec<&str> = OWN_KEYS.to_vec();
    let written: Vec<String> = sqlx::query_scalar(
        "INSERT INTO ui_messages (key, locale, value, source_hash, source_text, updated_by, updated_at)
         SELECT k.key, 'ru', t.value, k.source_hash, k.source_text, NULL, NOW()
         FROM UNNEST($1::text[], $2::text[]) AS t(key, value)
         JOIN ui_message_keys k ON k.key = t.key
         ON CONFLICT (key, locale) DO UPDATE
             SET value = EXCLUDED.value, source_hash = EXCLUDED.source_hash,
                 source_text = EXCLUDED.source_text, updated_at = NOW()
             WHERE ui_messages.key = ANY($3::text[]) AND ui_messages.value IS DISTINCT FROM EXCLUDED.value
         RETURNING key",
    )
    .bind(&keys)
    .bind(&values)
    .bind(&own)
    .fetch_all(pool)
    .await?;
    let known: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ui_message_keys WHERE key = ANY($1::text[])")
            .bind(&keys)
            .fetch_one(pool)
            .await?;
    // The section's own texts an earlier loader wrote: the shipped file is the source now.
    sqlx::query(
        "DELETE FROM ui_messages WHERE locale = 'ru' AND key LIKE 'translators.%' AND updated_by IS NULL",
    )
    .execute(pool)
    .await?;
    let total = i32::try_from(texts.len()).unwrap_or(i32::MAX);
    let out = SiteTextsLoaded {
        total,
        written: i32::try_from(written.len()).unwrap_or(i32::MAX),
        unknown: total - i32::try_from(known).unwrap_or(0),
    };
    tracing::info!(
        total = out.total,
        written = out.written,
        unknown = out.unknown,
        "translators site texts loaded"
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_a_map_of_strings_with_the_menu_key() {
        let t = texts();
        assert!(t.len() > 100);
        for key in OWN_KEYS {
            assert!(t.contains_key(*key), "{key} is in site-ru.json");
        }
    }
}
