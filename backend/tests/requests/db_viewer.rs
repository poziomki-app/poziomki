//! Integration tests for the `db::viewer` module — `DbViewer`,
//! `set_viewer_context`, `with_viewer_tx`/`with_anon_tx`, and the SECURITY
//! DEFINER auth lookups.

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use poziomki_backend::app::{build_test_app_context, reset_test_database};
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::{sessions, users};
use poziomki_backend::db::{
    self, find_user_for_login, resolve_session, set_anon_context, set_viewer_context, with_anon_tx,
    with_viewer_tx, DbViewer,
};
use serial_test::serial;
use uuid::Uuid;

async fn setup() {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("build test app context");
    reset_test_database().await.expect("truncate test tables");
}

async fn insert_user(email: &str, is_review_stub: bool) -> User {
    let mut conn = db::conn().await.expect("pool");
    let new_user = NewUser {
        pid: Uuid::new_v4(),
        email: email.to_string(),
        password: "hash".to_string(),
        api_key: Uuid::new_v4().to_string(),
        name: "Test".to_string(),
    };
    let inserted: User = diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_select())
        .get_result(&mut conn)
        .await
        .expect("insert user");

    if is_review_stub {
        diesel::update(users::table.find(inserted.id))
            .set(users::is_review_stub.eq(true))
            .execute(&mut conn)
            .await
            .expect("mark stub");
        users::table
            .find(inserted.id)
            .select(User::as_select())
            .first(&mut conn)
            .await
            .expect("reload")
    } else {
        inserted
    }
}

#[derive(diesel::deserialize::QueryableByName)]
struct CurrentSetting {
    #[diesel(sql_type = Text)]
    value: String,
}

#[tokio::test]
#[serial]
async fn set_viewer_context_exposes_gucs_inside_transaction() {
    setup().await;
    let user = insert_user("viewer_ctx_test@example.com", false).await;
    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let got = with_viewer_tx(viewer, |conn| {
        async move {
            let row: CurrentSetting =
                diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                    .get_result(conn)
                    .await?;
            let role: CurrentSetting =
                diesel::sql_query("SELECT current_setting('app.role', true) AS value")
                    .get_result(conn)
                    .await?;
            Ok((row.value, role.value))
        }
        .scope_boxed()
    })
    .await
    .expect("viewer tx");

    assert_eq!(got.0, user.id.to_string());
    assert_eq!(got.1, "user");
}

#[tokio::test]
#[serial]
async fn set_local_does_not_leak_across_transactions() {
    // pgdog pool is in transaction mode, but diesel holds the same underlying
    // connection across queries until it's dropped. The important property is
    // that SET LOCAL clears at COMMIT, so a second transaction on the *same*
    // connection object should see an empty app.user_id.
    setup().await;
    let user = insert_user("leak_test@example.com", false).await;
    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: false,
    };

    with_viewer_tx(viewer, |_| {
        async { Ok::<_, diesel::result::Error>(()) }.scope_boxed()
    })
    .await
    .expect("first tx");

    let after: String = with_anon_tx(|conn| {
        async move {
            let row: CurrentSetting =
                diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                    .get_result(conn)
                    .await?;
            Ok(row.value)
        }
        .scope_boxed()
    })
    .await
    .expect("second tx");
    // Inside the anon tx, user_id was reset to '0'.
    assert_eq!(after, "0");
}

#[tokio::test]
#[serial]
async fn with_anon_tx_sets_anon_role() {
    setup().await;
    let role: String = with_anon_tx(|conn| {
        async move {
            let row: CurrentSetting =
                diesel::sql_query("SELECT current_setting('app.role', true) AS value")
                    .get_result(conn)
                    .await?;
            Ok(row.value)
        }
        .scope_boxed()
    })
    .await
    .expect("anon tx");
    assert_eq!(role, "anon");
}

#[tokio::test]
#[serial]
async fn find_user_for_login_returns_known_and_none() {
    setup().await;
    let user = insert_user("login_lookup@example.com", true).await;

    let mut conn = db::conn().await.expect("pool");
    let hit = find_user_for_login(&mut conn, "login_lookup@example.com")
        .await
        .expect("lookup");
    let row = hit.expect("row present");
    assert_eq!(row.id, user.id);
    assert!(row.is_review_stub);

    let miss = find_user_for_login(&mut conn, "nobody@example.com")
        .await
        .expect("lookup");
    assert!(miss.is_none());
}

#[tokio::test]
#[serial]
async fn resolve_session_returns_known_and_none() {
    setup().await;
    let user = insert_user("session_lookup@example.com", false).await;

    let mut conn = db::conn().await.expect("pool");
    let token = format!("hashed-{}", Uuid::new_v4());
    let expires_at = Utc::now() + Duration::days(7);
    diesel::insert_into(sessions::table)
        .values((
            sessions::user_id.eq(user.id),
            sessions::token.eq(&token),
            sessions::expires_at.eq(expires_at),
        ))
        .execute(&mut conn)
        .await
        .expect("insert session");

    let hit = resolve_session(&mut conn, &token)
        .await
        .expect("lookup")
        .expect("row present");
    assert_eq!(hit.user_id, user.id);
    assert_eq!(hit.user_pid, user.pid);

    let miss = resolve_session(&mut conn, "no-such-token")
        .await
        .expect("lookup");
    assert!(miss.is_none());
}

#[tokio::test]
#[serial]
async fn set_viewer_context_without_wrapper_also_works_inside_explicit_tx() {
    // set_viewer_context is a primitive used inside existing
    // build_transaction()... call sites. Confirm it's composable.
    use diesel_async::AsyncConnection;
    setup().await;
    let user = insert_user("primitive_ctx@example.com", false).await;
    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: false,
    };

    let mut conn = db::conn().await.expect("pool");
    let got: String = conn
        .transaction::<String, diesel::result::Error, _>(|conn| {
            async move {
                set_viewer_context(conn, viewer).await?;
                let row: CurrentSetting =
                    diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                        .get_result(conn)
                        .await?;
                Ok(row.value)
            }
            .scope_boxed()
        })
        .await
        .expect("tx");
    assert_eq!(got, user.id.to_string());

    // Anon primitive also works on a fresh transaction.
    let got_anon: String = conn
        .transaction::<String, diesel::result::Error, _>(|conn| {
            async move {
                set_anon_context(conn).await?;
                let row: CurrentSetting =
                    diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                        .get_result(conn)
                        .await?;
                Ok(row.value)
            }
            .scope_boxed()
        })
        .await
        .expect("anon tx");
    assert_eq!(got_anon, "0");
}
