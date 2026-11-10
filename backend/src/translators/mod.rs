//! The translators' section ("Для переводчиков"): a read-only EN | RU | CN
//! reader of the story text, with Sova's and the AI's translations as hints
//! for signed-in translators.
//!
//! Kept apart from the upstream code so the fork merges cleanly: its own
//! module, routes under `/api/translators`, tables in the `translators`
//! schema with their own migration ledger. The upstream files only carry the
//! three hooks: `pub mod translators` (lib.rs), `.merge(translators::router())`
//! (routes) and `translators::db::migrate` (database init).

pub mod db;
pub mod dto;
pub mod edits;
pub mod export;
pub mod glossary;
pub mod import;
pub mod routes;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::state::AppState;

/// The server whose scripts the section reads.
pub const SERVER: &str = "en";

/// `TRANSLATORS_DEV_LOGIN=1` switches the development sign-in on. Off unless set.
pub fn dev_login_enabled() -> bool {
    std::env::var("TRANSLATORS_DEV_LOGIN").is_ok_and(|v| v.trim() == "1")
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(routes::me))
        .routes(routes!(routes::set_recommend_rule))
        .routes(routes!(routes::story_lines))
        .routes(routes!(routes::run_import, routes::import_status))
        .routes(routes!(routes::dev_login))
        .routes(routes!(routes::glossary))
        .routes(routes!(routes::glossary_lines))
        .routes(routes!(routes::refresh_glossary))
        .routes(routes!(routes::export))
        .routes(routes!(routes::export_archive))
        .routes(routes!(routes::mark_release, routes::list_releases))
        .merge(edits::router())
}
