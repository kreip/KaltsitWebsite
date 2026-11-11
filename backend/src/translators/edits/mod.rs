//! Line editing: proposals, "keep as in the
//! game", comments and "My edits". Everything is keyed by the English of the
//! dictionary entry (`story_lines.edit_key`), so one decision holds wherever
//! the line stands.

pub mod db;
pub mod dto;
pub mod routes;
pub mod status;

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::state::AppState;
use crate::translators::dto::TranslatorLine;
use dto::{EditPlace, KeyEdits, OutdatedEdit};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(routes::create_proposal))
        .routes(routes!(routes::rewrite_proposal, routes::delete_proposal))
        .routes(routes!(routes::proposal_versions))
        .routes(routes!(routes::line_history))
        .routes(routes!(routes::set_agreement))
        .routes(routes!(routes::create_comment))
        .routes(routes!(routes::delete_comment))
        .routes(routes!(routes::resolve_comment))
        .routes(routes!(routes::places))
        .routes(routes!(routes::edits))
        .routes(routes!(routes::characters))
        .routes(routes!(routes::names))
        .routes(routes!(routes::set_story_read))
        .routes(routes!(routes::story_progress))
        .routes(routes!(routes::set_approval))
        .routes(routes!(routes::approve_batch))
}

/// Fill each line's places and status, and return the edits for the
/// story's entries and the decisions left at its changed places.
pub async fn attach(
    pool: &PgPool,
    story_id: &str,
    lines: &mut [TranslatorLine],
) -> Result<(Vec<KeyEdits>, Vec<OutdatedEdit>), sqlx::Error> {
    let outdated = db::outdated_in_story(pool, story_id).await?;
    let stale: HashSet<(i32, i16)> = outdated.iter().map(|o| (o.line, o.part)).collect();
    let places: Vec<EditPlace> = lines
        .iter()
        .map(|l| EditPlace {
            story_id: story_id.to_owned(),
            line: i32::try_from(l.line).unwrap_or(0),
            part: l.part,
            edit_key: None,
        })
        .collect();
    let edits = attach_at(pool, &places, lines, |p| stale.contains(&(p.line, p.part))).await?;
    Ok((edits, outdated))
}

fn same_place(a: &EditPlace, b: &EditPlace) -> bool {
    a.story_id == b.story_id && a.line == b.line && a.part == b.part
}

/// Fill the places and status of lines that each stand at their own place
/// (a story's lines, or the search's), and return the edits for their entries.
/// `stale`: a decision for an English that has changed is left at the place.
pub async fn attach_at(
    pool: &PgPool,
    places: &[EditPlace],
    lines: &mut [TranslatorLine],
    stale: impl Fn(&EditPlace) -> bool,
) -> Result<Vec<KeyEdits>, sqlx::Error> {
    let keys: Vec<String> = lines
        .iter()
        .map(|l| l.edit_key.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let counts = db::place_counts(pool, &keys).await?;
    let edits = db::key_edits(pool, &keys).await?;
    let before = db::ru_before(pool, &keys).await?;
    let by_key: HashMap<&str, &KeyEdits> = edits.iter().map(|e| (e.edit_key.as_str(), e)).collect();
    for (l, here) in lines.iter_mut().zip(places) {
        l.places = counts.get(&l.edit_key).copied().unwrap_or(1).max(1);
        if let Some((text, at)) = before.get(&l.edit_key) {
            l.ru_game_before = Some(text.clone());
            l.ru_game_changed_at = Some(at.timestamp());
        }
        let e = by_key.get(l.edit_key.as_str());
        // A proposal for one place counts only there.
        let shows = |p: &&dto::Proposal| !p.context || same_place(&p.place, here);
        // Approved: the entry, or a one-place proposal here.
        let approved_here = e.is_some_and(|e| {
            e.approval.is_some()
                || e.proposals
                    .iter()
                    .any(|p| p.context && p.approved_at.is_some() && same_place(&p.place, here))
        });
        l.status = status::line_status(
            approved_here,
            stale(here),
            e.map_or(0, |e| e.proposals.iter().filter(shows).count()),
        )
        .as_str()
        .to_owned();
    }
    Ok(edits)
}
