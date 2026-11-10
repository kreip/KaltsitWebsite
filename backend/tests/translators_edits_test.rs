//! Line editing of the translators' section against a real Postgres: the
//! section's migrations apply, a proposal is written, rewritten (with its
//! history), voted on and deleted, comments resolve, the story's edits and
//! "My edits" read back, and an import that drops the English marks them
//! outdated.
//!
//! Makes a throwaway database next to the one in `TRANSLATORS_TEST_DB`
//! (default: the local compose database), with a minimal `users` table, and
//! drops it at the end. Ignored by default:
//! `cargo test --test translators_edits_test -- --ignored --nocapture`

use backend::translators::edits::db;
use backend::translators::edits::dto::EditPlace;
use backend::translators::{db as tdb, edits};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

fn admin_url() -> String {
    std::env::var("TRANSLATORS_TEST_DB")
        .unwrap_or_else(|_| "postgres://myrtle:myrtle@127.0.0.1:5432/myrtle".to_owned())
}

async fn fresh_db() -> (PgPool, String) {
    let admin = admin_url();
    let name = format!("tr_edits_{}", &Uuid::new_v4().simple().to_string()[..10]);
    let mut conn = PgConnection::connect(&admin)
        .await
        .expect("admin connection");
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(&mut conn)
        .await
        .expect("create database");
    let base = admin.rsplit_once('/').expect("db url").0;
    let pool = PgPool::connect(&format!("{base}/{name}"))
        .await
        .expect("test pool");
    sqlx::raw_sql(
        "CREATE TABLE users (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), uid TEXT NOT NULL,
                             server_id SMALLINT NOT NULL DEFAULT 0, nickname TEXT)",
    )
    .execute(&pool)
    .await
    .expect("users");
    tdb::migrate(&pool).await.expect("translators migrations");
    (pool, name)
}

async fn drop_db(pool: PgPool, name: String) {
    pool.close().await;
    let mut conn = PgConnection::connect(&admin_url()).await.expect("admin");
    sqlx::raw_sql(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&mut conn)
        .await
        .expect("drop database");
}

async fn user(pool: &PgPool, uid: &str) -> Uuid {
    let id = sqlx::query_scalar("INSERT INTO users (uid, nickname) VALUES ($1, $1) RETURNING id")
        .bind(uid)
        .fetch_one(pool)
        .await
        .expect("user");
    tdb::add_member(pool, id, "test").await.expect("member");
    id
}

async fn line(pool: &PgPool, story: &str, line: i32, ord: i32, en: &str, ru: &str) {
    sqlx::query(
        "INSERT INTO translators.story_lines (server, story_id, line, part, ord, kind, en, ru_game, edit_key)
         VALUES ('en', $1, $2, 0, $3, 'dialog', $4, $5, $4)",
    )
    .bind(story)
    .bind(line)
    .bind(ord)
    .bind(en)
    .bind(ru)
    .execute(pool)
    .await
    .expect("line");
}

fn place(story: &str, line: i32) -> EditPlace {
    EditPlace {
        story_id: story.to_owned(),
        line,
        part: 0,
    }
}

#[tokio::test]
#[ignore = "creates and drops a database on the local Postgres"]
async fn proposals_votes_comments_and_outdated() {
    let (pool, name) = fresh_db().await;
    let a = user(&pool, "anna").await;
    let b = user(&pool, "boris").await;
    assert_eq!(
        tdb::member_role(&pool, a).await.unwrap().map(|r| r.0),
        Some(vec!["translator".to_owned()])
    );

    line(&pool, "s1", 3, 0, "Doctor?", "Доктор?").await;
    line(&pool, "s1", 5, 1, "Hello.", "Привет.").await;
    line(&pool, "s2", 7, 4, "Hello.", "Привет.").await;
    let key = "Hello.".to_owned();

    let counts = db::place_counts(&pool, std::slice::from_ref(&key))
        .await
        .unwrap();
    assert_eq!(counts.get("Hello."), Some(&2));
    let places = db::places(&pool, &key, 10).await.unwrap();
    assert_eq!(
        places[0].before.as_deref(),
        Some("Доктор?"),
        "the line before, in Russian"
    );
    assert_eq!(places[1].after, None);
    assert_eq!(
        places
            .iter()
            .map(|p| (p.story_id.as_str(), p.ord))
            .collect::<Vec<_>>(),
        vec![("s1", 1), ("s2", 4)]
    );

    // Anna writes, then takes Sova's: one proposal, two versions.
    let p1 = db::upsert_proposal(&pool, &key, &place("s1", 5), "Здравствуй.", "own", false, a)
        .await
        .unwrap();
    let again = db::upsert_proposal(
        &pool,
        &key,
        &place("s2", 7),
        "Здравствуйте.",
        "sova",
        false,
        a,
    )
    .await
    .unwrap();
    assert_eq!(p1, again);
    db::rewrite_proposal(&pool, p1, "Добрый день.", "sova", a)
        .await
        .unwrap();
    let versions = db::versions(&pool, p1).await.unwrap();
    assert_eq!(
        versions.iter().map(|v| v.text.as_str()).collect::<Vec<_>>(),
        vec!["Добрый день.", "Здравствуйте.", "Здравствуй."]
    );
    let p2 = db::upsert_proposal(&pool, &key, &place("s1", 5), "Хай.", "own", false, b)
        .await
        .unwrap();
    assert_ne!(p1, p2);

    // Boris agrees with Anna's; Anna keeps the game's Russian too.
    db::set_agreement(
        &pool,
        &db::Vote {
            key: &key,
            variant: "proposal",
            proposal: Some(p1),
            place: None,
            text: None,
        },
        b,
        true,
    )
    .await
    .unwrap();
    db::set_agreement(
        &pool,
        &db::Vote {
            key: &key,
            variant: "proposal",
            proposal: Some(p1),
            place: None,
            text: None,
        },
        b,
        true,
    )
    .await
    .unwrap();
    db::set_agreement(
        &pool,
        &db::Vote {
            key: &key,
            variant: "game",
            proposal: None,
            place: None,
            text: None,
        },
        a,
        true,
    )
    .await
    .unwrap();
    let c = db::add_comment(
        &pool,
        &key,
        &place("s1", 5),
        &db::NewComment {
            text: "Слишком официально?",
            quote: Some("Хай"),
            kind: "remark",
            variant: None,
            proposal_id: None,
            variant_text: None,
        },
        b,
    )
    .await
    .unwrap();
    db::resolve_comment(&pool, c, Some(a)).await.unwrap();

    let mut lines = tdb::story_lines(&pool, "en", "s1", true).await.unwrap();
    let (got, outdated) = edits::attach(&pool, "s1", &mut lines).await.unwrap();
    assert!(outdated.is_empty());
    assert_eq!(lines[0].status, "unchecked");
    assert_eq!(lines[1].status, "proposed");
    assert_eq!(lines[1].places, 2);
    let e = got.iter().find(|e| e.edit_key == "Hello.").unwrap();
    assert_eq!(e.proposals.len(), 2);
    assert_eq!(e.proposals[0].id, p1, "the most agreed comes first");
    assert_eq!(e.proposals[0].agreements.len(), 1);
    assert_eq!(e.proposals[0].versions, 3);
    assert!(e.proposals[0].edited_at.is_some());
    assert_eq!(e.proposals[0].source, "sova");
    assert_eq!(e.game_agreements.len(), 1);
    assert_eq!(e.comments[0].resolved_by.as_ref().unwrap().name, "anna");
    assert_eq!(e.comments[0].quote.as_deref(), Some("Хай"));

    // A proposal for one place: a separate proposal, counted only there.
    let pc = db::upsert_proposal(
        &pool,
        &key,
        &place("s2", 7),
        "Привет-привет.",
        "own",
        true,
        b,
    )
    .await
    .unwrap();
    assert_ne!(pc, p2);
    let mut s2 = tdb::story_lines(&pool, "en", "s2", true).await.unwrap();
    edits::attach(&pool, "s2", &mut s2).await.unwrap();
    let mut s1 = tdb::story_lines(&pool, "en", "s1", true).await.unwrap();
    let (got1, _) = edits::attach(&pool, "s1", &mut s1).await.unwrap();
    assert_eq!(s1[1].status, "proposed");
    let ctx = got1[0].proposals.iter().find(|p| p.id == pc).unwrap();
    assert!(ctx.context);
    assert_eq!(ctx.place.story_id, "s2");
    db::delete_proposal(&pool, pc).await.unwrap();

    let mine = db::edit_rows(&pool, Some(a)).await.unwrap();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].ru_game.as_deref(), Some("Привет."));
    assert_eq!(mine[0].state, "proposed");
    assert_eq!(mine[0].places, 2);
    assert_eq!(
        mine[0].ord,
        Some(4),
        "the place moved to where it was rewritten"
    );
    assert_eq!(db::edit_rows(&pool, None).await.unwrap().len(), 2);

    // Boris deletes his; the vote for Anna's stays, a vote for his would go.
    db::set_agreement(
        &pool,
        &db::Vote {
            key: &key,
            variant: "proposal",
            proposal: Some(p2),
            place: None,
            text: None,
        },
        a,
        true,
    )
    .await
    .unwrap();
    db::delete_proposal(&pool, p2).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translators.agreements")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 2);

    // A vote for Sova's text at one place, a read, and the progress.
    let at = place("s1", 3);
    db::set_agreement(
        &pool,
        &db::Vote {
            key: "Doctor?",
            variant: "sova",
            proposal: None,
            place: Some(&at),
            text: Some("Доктор?"),
        },
        b,
        true,
    )
    .await
    .unwrap();
    let got = db::key_edits(&pool, &["Doctor?".to_owned()]).await.unwrap();
    assert_eq!(got[0].place_agreements[0].variant, "sova");
    assert_eq!(got[0].place_agreements[0].people[0].name, "boris");
    db::set_read(&pool, "s1", a, true).await.unwrap();
    db::set_read(&pool, "s1", a, true).await.unwrap();
    assert_eq!(db::story_reads(&pool, "s1").await.unwrap().len(), 1);
    db::set_read(&pool, "s1", a, false).await.unwrap();
    assert!(db::story_reads(&pool, "s1").await.unwrap().is_empty());
    let progress = db::progress(&pool).await.unwrap();
    let s1 = progress.iter().find(|p| p.story_id == "s1").unwrap();
    assert_eq!(
        (s1.total, s1.approved, s1.waiting, s1.outdated),
        (2, 0, 1, 0)
    );

    // An import changes the English at s1:5 and s2:7: the decisions go stale.
    sqlx::query(
        "UPDATE translators.story_lines SET en = 'Hi.', edit_key = 'Hi.' WHERE en = 'Hello.'",
    )
    .execute(&pool)
    .await
    .unwrap();
    db::mark_outdated(&pool).await.unwrap();
    let mut lines = tdb::story_lines(&pool, "en", "s2", true).await.unwrap();
    let (_, outdated) = edits::attach(&pool, "s2", &mut lines).await.unwrap();
    assert_eq!(lines[0].status, "outdated");
    assert_eq!(outdated[0].edit_key, "Hello.");
    assert_eq!(
        db::edit_rows(&pool, Some(a)).await.unwrap()[0].state,
        "outdated"
    );
    // It comes back: the mark goes.
    sqlx::query(
        "UPDATE translators.story_lines SET en = 'Hello.', edit_key = 'Hello.' WHERE en = 'Hi.'",
    )
    .execute(&pool)
    .await
    .unwrap();
    db::mark_outdated(&pool).await.unwrap();
    assert_eq!(
        db::edit_rows(&pool, Some(a)).await.unwrap()[0].state,
        "proposed"
    );

    drop_db(pool, name).await;
}

#[tokio::test]
#[ignore = "creates and drops a database on the local Postgres"]
async fn approvals_progress_and_export_rows() {
    use backend::translators::export;
    let (pool, name) = fresh_db().await;
    let lead = user(&pool, "lead").await;
    let a = user(&pool, "anna").await;
    line(&pool, "s1", 1, 0, "Hello.", "Привет.").await;
    line(&pool, "s1", 2, 1, "Bye.", "Пока.").await;
    line(&pool, "s1", 3, 2, "Yes.", "Да.").await;
    let at = |l| place("s1", l);

    // A proposal approved for its entry, the game's Russian checked, a one-place proposal approved there.
    let p = db::upsert_proposal(&pool, "Hello.", &at(1), "Здравствуй.", "own", false, a)
        .await
        .unwrap();
    db::approve_key(
        &pool,
        "Hello.",
        "proposal",
        Some(p),
        "Здравствуй.",
        &at(1),
        lead,
    )
    .await
    .unwrap();
    db::approve_key(&pool, "Bye.", "game", None, "Пока.", &at(2), lead)
        .await
        .unwrap();
    let c = db::upsert_proposal(&pool, "Yes.", &at(3), "Ага.", "own", true, a)
        .await
        .unwrap();
    db::approve_context(&pool, "Yes.", c, &at(3), lead, true)
        .await
        .unwrap();

    let s1 = db::progress(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.story_id == "s1")
        .unwrap();
    assert_eq!((s1.total, s1.approved, s1.waiting), (3, 3, 0));
    let mut lines = tdb::story_lines(&pool, "en", "s1", true).await.unwrap();
    edits::attach(&pool, "s1", &mut lines).await.unwrap();
    assert!(lines.iter().all(|l| l.status == "approved"));

    // For the game: only the proposal (the game's Russian checked never goes); the one-place one to context.
    let rows = export::rows(&pool, false).await.unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| (r.edit_key.as_str(), r.text.as_str()))
            .collect::<Vec<_>>(),
        vec![("Hello.", "Здравствуй.")]
    );
    let ctx = export::context_rows(&pool, false).await.unwrap();
    assert_eq!((ctx[0].text.as_str(), ctx[0].ord), ("Ага.", Some(2)));

    // Withdrawn: back to waiting, and in the review export.
    db::withdraw_key(&pool, "Hello.", &at(1), lead)
        .await
        .unwrap();
    let s1 = db::progress(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.story_id == "s1")
        .unwrap();
    assert_eq!((s1.approved, s1.waiting), (2, 1));
    let review = export::rows(&pool, true).await.unwrap();
    assert_eq!(review[0].edit_key, "Hello.");
    let logged: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translators.approval_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(logged, 4);

    // A pack release marks what is approved now (the one-place proposal), once.
    let r1 = export::mark_release(&pool, "1.0.30", None, lead)
        .await
        .unwrap();
    let r2 = export::mark_release(&pool, "1.0.31", None, lead)
        .await
        .unwrap();
    let all = export::releases(&pool).await.unwrap();
    let count = |id| {
        all.iter()
            .find(|r| r.id == id)
            .map(|r| (r.dictionary, r.context))
            .unwrap()
    };
    assert_eq!(count(r1), (0, 1));
    assert_eq!(count(r2), (0, 0), "nothing new");
    let got = db::key_edits(&pool, &["Yes.".to_owned()]).await.unwrap();
    assert_eq!(got[0].proposals[0].released_in.as_deref(), Some("1.0.30"));
    drop_db(pool, name).await;
}

/// Comparing versions: an import that brings a new Russian keeps the one it
/// replaced, a rewritten proposal knows its previous text, and the line's
/// history lists every step newest first with the text one step back.
#[tokio::test]
#[ignore = "creates and drops a database on the local Postgres"]
async fn history_of_a_line() {
    use backend::translators::import::{self, lines::Line};
    let (pool, name) = fresh_db().await;
    let lead = user(&pool, "lead").await;
    let a = user(&pool, "anna").await;
    let built = |ru: &str| import::Built {
        stories: vec![(
            "s1".to_owned(),
            vec![
                Line {
                    line: 1,
                    part: 0,
                    kind: "dialog",
                    speaker_en: String::new(),
                    speaker_ru: None,
                    en: "Hello.".to_owned(),
                    ru_game: Some(ru.to_owned()),
                    ru_game_source: None,
                    ru_game_match: None,
                    plugin_role: None,
                    ru_dict: None,
                    speaker_cn: None,
                    cn: None,
                    sova: None,
                    ai: None,
                    arktr_match: None,
                    edit_key: "Hello.".to_owned(),
                },
                Line {
                    line: 2,
                    part: 0,
                    kind: "dialog",
                    speaker_en: String::new(),
                    speaker_ru: None,
                    en: "Bye.".to_owned(),
                    ru_game: Some("Пока.".to_owned()),
                    ru_game_source: None,
                    ru_game_match: None,
                    plugin_role: None,
                    ru_dict: None,
                    speaker_cn: None,
                    cn: None,
                    sova: None,
                    ai: None,
                    arktr_match: None,
                    edit_key: "Bye.".to_owned(),
                },
            ],
        )],
        chars: Vec::new(),
        sova_notes: Vec::new(),
        names_ru: Vec::new(),
        summary: import::ImportSummary::default(),
    };
    let keys = ["Hello.".to_owned(), "Bye.".to_owned()];
    let t0 = chrono::Utc::now() - chrono::Duration::days(2);

    // The first import and a repeat of it change nothing.
    import::store(&pool, "en", t0, &built("Привет."))
        .await
        .unwrap();
    import::store(&pool, "en", t0, &built("Привет."))
        .await
        .unwrap();
    assert!(db::ru_before(&pool, &keys).await.unwrap().is_empty());

    // A new pack: only the changed entry keeps its old Russian.
    let t1 = chrono::Utc::now() - chrono::Duration::days(1);
    import::store(&pool, "en", t1, &built("Здравствуй."))
        .await
        .unwrap();
    let before = db::ru_before(&pool, &keys).await.unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before["Hello."].0, "Привет.");
    assert_eq!(before["Hello."].1.timestamp(), t1.timestamp());

    // Anna proposes, rewrites; the lead approves.
    let here = place("s1", 1);
    let p = db::upsert_proposal(&pool, "Hello.", &here, "Добрый день.", "own", false, a)
        .await
        .unwrap();
    let got = db::key_edits(&pool, &keys[..1]).await.unwrap();
    assert_eq!(got[0].proposals[0].previous_text, None, "never rewritten");
    db::rewrite_proposal(&pool, p, "Добрый вечер.", "own", a)
        .await
        .unwrap();
    let got = db::key_edits(&pool, &keys[..1]).await.unwrap();
    assert_eq!(
        got[0].proposals[0].previous_text.as_deref(),
        Some("Добрый день.")
    );
    db::approve_key(
        &pool,
        "Hello.",
        "proposal",
        Some(p),
        "Добрый вечер.",
        &here,
        lead,
    )
    .await
    .unwrap();
    // A proposal for another place only stays out of this place's history.
    line(&pool, "s2", 9, 0, "Hello.", "Здравствуй.").await;
    db::upsert_proposal(&pool, "Hello.", &place("s2", 9), "Салют.", "own", true, a)
        .await
        .unwrap();

    let events = db::history(&pool, "Hello.", &here, Some("Здравствуй."))
        .await
        .unwrap();
    let steps: Vec<(&str, Option<&str>, Option<&str>)> = events
        .iter()
        .map(|e| (e.kind.as_str(), e.before.as_deref(), e.text.as_deref()))
        .collect();
    assert_eq!(
        steps,
        vec![
            ("approve", None, Some("Добрый вечер.")),
            ("proposal", Some("Добрый день."), Some("Добрый вечер.")),
            ("proposal", Some("Здравствуй."), Some("Добрый день.")),
            ("game", Some("Привет."), Some("Здравствуй.")),
        ]
    );
    let there = db::history(&pool, "Hello.", &place("s2", 9), Some("Здравствуй."))
        .await
        .unwrap();
    assert!(
        there
            .iter()
            .any(|e| e.context && e.text.as_deref() == Some("Салют.")),
        "the one-place proposal shows at its own place"
    );
    drop_db(pool, name).await;
}
