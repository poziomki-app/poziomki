//! Baseline + privilege tests for the RLS rollout.
//!
//! These tests run *before* any policy migration lands. They do two jobs:
//! 1. Assert the surrounding plumbing is correct — least-privilege roles,
//!    SD helpers hardened, API role has DML grants on the expected tables.
//! 2. Act as a **canary**: the "no policy enabled yet" assertions will
//!    fail once a Tier-A/B/C policy PR flips `RLS ENABLE` on a given
//!    table. That failure is intentional — it forces the policy PR
//!    author to update the expected set here alongside their migration.

use poziomki_backend::app::build_test_app_context;
use serial_test::serial;

use super::rls_harness;

/// Every table the plan will eventually protect under an RLS policy.
/// When a Tier-A/B/C migration lands, move the table from
/// `EXPECTED_RLS_DISABLED_TABLES` into the tier-specific expected set in
/// that PR's test file.
/// Tables that have Tier-A RLS policies attached (user/profile-owned
/// rows). Each must have RLS enabled, FORCE set, and a policy named
/// `<table>_viewer` in schema public. Moved here from the "disabled"
/// canary in the Tier-A migration 2026-04-19-010000.
const EXPECTED_TIER_A_TABLES: &[&str] = &[
    "users",
    "profiles",
    "profile_tags",
    "sessions",
    "user_settings",
    "user_audit_log",
    "push_subscriptions",
    "xp_scans",
    "task_completions",
    "profile_bookmarks",
    "profile_blocks",
    "recommendation_feedback",
    "event_interactions",
    "reports",
];

/// Tables the Tier-B migration (2026-04-19-020000) locks down. Same
/// shape as the Tier-A set: each table must have RLS enabled, FORCE
/// set, and a `<table>_viewer` SELECT policy on `poziomki_api`.
const EXPECTED_TIER_B_TABLES: &[&str] = &[
    "conversations",
    "conversation_members",
    "messages",
    "message_reactions",
];

/// Tier-C tables (events, attendance, uploads). Same shape as Tier-A
/// and Tier-B: ENABLED + FORCED + `<table>_viewer` SELECT policy on
/// `poziomki_api`.
const EXPECTED_TIER_C_TABLES: &[&str] = &["events", "event_attendees", "uploads"];

/// No tables remaining in the "RLS still off" canary — every table
/// the plan protects is now locked down.
const EXPECTED_RLS_DISABLED_TABLES: &[&str] = &[];

/// SD helpers installed by the 010000..070000 migration series. Every
/// entry must carry `search_path=pg_catalog, pg_temp` so the `pg_temp`
/// hijack mitigation from migration 060000 stays in effect.
const EXPECTED_SD_HELPERS: &[&str] = &[
    "award_profile_xp",
    "complete_password_reset",
    "create_session_for_user",
    "create_user_for_signup",
    "delete_session_by_token",
    "find_user_for_login",
    "find_user_for_password_reset",
    "mark_email_verified",
    "profile_owner_user_id",
    "profile_program_visibility",
    // Tier-A policy-support helpers. These return narrow viewer-scoped
    // row sets and are SECURITY DEFINER so policy expressions that embed
    // them don't recursively self-filter against RLS.
    "profiles_in_current_bucket",
    "push_subscriptions_for_users",
    "resolve_session",
    // Tier-C policy-support helpers.
    "event_auto_approves",
    "viewer_owns_event",
    // Tier-B policy-support helpers.
    "conversation_meta_for_insert",
    "delete_event_and_chat",
    "event_creator_user_id",
    "find_dm_conversation",
    "find_event_conversation",
    "viewer_can_access_event",
    "viewer_can_see_message",
    "viewer_conversation_ids",
    "set_password_reset_token",
    "user_id_for_pid",
    "user_pid_for_id",
    "user_pids_for_ids",
    "user_review_stubs",
    "viewer_profile_ids",
];

/// Non-SECURITY-DEFINER helpers in schema `app`. These only consult GUCs
/// (no table reads), so they don't need definer rights. Listed
/// separately so a dropped helper surfaces loudly even though it won't
/// appear in the hardened-search_path test.
const EXPECTED_POLICY_HELPERS: &[&str] =
    &["current_is_stub", "current_user_id", "session_bypasses_rls"];

fn setup() {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("build test app context");
}

#[tokio::test]
#[serial]
async fn api_role_is_least_privilege() {
    setup();
    let (bypass, can_login) = rls_harness::role_flags("poziomki_api").await;
    assert!(
        can_login,
        "poziomki_api must be a login role (the API connects as it)"
    );
    assert!(
        !bypass,
        "poziomki_api must be NOBYPASSRLS — otherwise future policies are ineffective"
    );
}

#[tokio::test]
#[serial]
async fn worker_role_has_bypassrls() {
    setup();
    let (bypass, can_login) = rls_harness::role_flags("poziomki_worker").await;
    assert!(can_login, "poziomki_worker must be a login role");
    assert!(
        bypass,
        "poziomki_worker needs BYPASSRLS for cross-user maintenance jobs"
    );
}

#[tokio::test]
#[serial]
async fn api_role_has_dml_on_all_protected_tables() {
    setup();
    let all_tables = EXPECTED_TIER_A_TABLES
        .iter()
        .chain(EXPECTED_TIER_B_TABLES.iter())
        .chain(EXPECTED_TIER_C_TABLES.iter())
        .chain(EXPECTED_RLS_DISABLED_TABLES.iter());
    for table in all_tables {
        let grants = rls_harness::role_privileges("poziomki_api", table).await;
        for privilege in &["SELECT", "INSERT", "UPDATE", "DELETE"] {
            assert!(
                grants.contains(*privilege),
                "poziomki_api missing {privilege} on public.{table} (found {grants:?})"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn every_sd_helper_uses_hardened_search_path() {
    setup();
    let configs = rls_harness::sd_function_configs().await;
    for name in EXPECTED_SD_HELPERS {
        let entry = configs
            .get(*name)
            .expect("SD helper is missing from schema `app`");
        let config = entry.as_deref().unwrap_or("");
        assert!(
            config.contains("search_path=pg_catalog, pg_temp"),
            "SD helper app.{name} must pin search_path=pg_catalog, pg_temp (got {config:?})"
        );
    }
}

/// Canary: every table in `EXPECTED_RLS_DISABLED_TABLES` currently has
/// RLS **off**. When a policy PR enables RLS on a table, this test fails
/// and forces the author to remove that table from the list and add the
/// corresponding tier-specific assertions in a new test file.
#[tokio::test]
#[serial]
async fn rls_is_not_yet_enabled_on_protected_tables() {
    setup();
    let mut unexpected = Vec::new();
    for table in EXPECTED_RLS_DISABLED_TABLES {
        if rls_harness::table_rls_enabled(table).await {
            unexpected.push(*table);
        }
    }
    assert!(
        unexpected.is_empty(),
        "Tables have RLS enabled but the baseline doesn't know about it — \
         move these into the tier-specific expected set and add policy \
         assertions there: {unexpected:?}"
    );
}

/// Symmetric canary for `FORCE ROW LEVEL SECURITY`. Tier-policy PRs use
/// `ENABLE + FORCE` together so the owner role doesn't bypass policies.
#[tokio::test]
#[serial]
async fn rls_is_not_yet_forced_on_protected_tables() {
    setup();
    let mut unexpected = Vec::new();
    for table in EXPECTED_RLS_DISABLED_TABLES {
        if rls_harness::table_force_rls(table).await {
            unexpected.push(*table);
        }
    }
    assert!(
        unexpected.is_empty(),
        "Tables have FORCE RLS set — move them into the tier-specific \
         expected set: {unexpected:?}"
    );
}

/// Every Tier-A table has `ROW LEVEL SECURITY` ENABLED and FORCED. If
/// the Tier-A migration ever regresses (a DROP POLICY sneaks in, a
/// DISABLE lands on a table) this test fails loudly instead of leaving
/// silent unprotected rows.
#[tokio::test]
#[serial]
async fn tier_a_tables_have_rls_enabled_and_forced() {
    setup();
    let mut missing_enabled = Vec::new();
    let mut missing_forced = Vec::new();
    for table in EXPECTED_TIER_A_TABLES {
        if !rls_harness::table_rls_enabled(table).await {
            missing_enabled.push(*table);
        }
        if !rls_harness::table_force_rls(table).await {
            missing_forced.push(*table);
        }
    }
    assert!(
        missing_enabled.is_empty(),
        "Tier-A tables missing RLS ENABLE: {missing_enabled:?}"
    );
    assert!(
        missing_forced.is_empty(),
        "Tier-A tables missing FORCE ROW LEVEL SECURITY (owner role would bypass): {missing_forced:?}"
    );
}

/// Every Tier-A table has a policy named `<table>_viewer` attached to
/// `poziomki_api`. Detects accidental policy drops or role-retargeting.
#[tokio::test]
#[serial]
async fn tier_a_tables_have_named_policy_on_api_role() {
    setup();
    let attachments = rls_harness::policies_for_tables(EXPECTED_TIER_A_TABLES).await;
    for table in EXPECTED_TIER_A_TABLES {
        let policies = attachments
            .get(*table)
            .expect("policy catalog query must return an entry per table");
        let expected_name = format!("{table}_viewer");
        let matched = policies.iter().find(|p| p.name == expected_name);
        assert!(
            matched.is_some(),
            "Tier-A table public.{table} is missing policy {expected_name}; found {policies:?}"
        );
        let matched = matched.unwrap();
        assert!(
            matched.roles.iter().any(|r| r == "poziomki_api"),
            "Tier-A policy {expected_name} on public.{table} must target poziomki_api (targets: {:?})",
            matched.roles
        );
    }
}

/// Tier-B canary: chat tables (`conversations`, `conversation_members`,
/// `messages`, `message_reactions`) have RLS ENABLED + FORCED.
#[tokio::test]
#[serial]
async fn tier_b_tables_have_rls_enabled_and_forced() {
    setup();
    let mut missing_enabled = Vec::new();
    let mut missing_forced = Vec::new();
    for table in EXPECTED_TIER_B_TABLES {
        if !rls_harness::table_rls_enabled(table).await {
            missing_enabled.push(*table);
        }
        if !rls_harness::table_force_rls(table).await {
            missing_forced.push(*table);
        }
    }
    assert!(
        missing_enabled.is_empty(),
        "Tier-B tables missing RLS ENABLE: {missing_enabled:?}"
    );
    assert!(
        missing_forced.is_empty(),
        "Tier-B tables missing FORCE ROW LEVEL SECURITY: {missing_forced:?}"
    );
}

/// Every Tier-B table has a `<table>_viewer` SELECT policy attached
/// to `poziomki_api`. Separate write policies exist too but the
/// read policy name is the stable canary handle.
#[tokio::test]
#[serial]
async fn tier_b_tables_have_named_policy_on_api_role() {
    setup();
    let attachments = rls_harness::policies_for_tables(EXPECTED_TIER_B_TABLES).await;
    for table in EXPECTED_TIER_B_TABLES {
        let policies = attachments
            .get(*table)
            .expect("policy catalog query must return an entry per table");
        let expected_name = format!("{table}_viewer");
        let matched = policies.iter().find(|p| p.name == expected_name);
        assert!(
            matched.is_some(),
            "Tier-B table public.{table} is missing policy {expected_name}; found {policies:?}"
        );
        let matched = matched.unwrap();
        assert!(
            matched.roles.iter().any(|r| r == "poziomki_api"),
            "Tier-B policy {expected_name} on public.{table} must target poziomki_api (targets: {:?})",
            matched.roles
        );
    }
}

/// Tier-C canary: `events`, `event_attendees`, `uploads` have RLS
/// ENABLED + FORCED + `<table>_viewer` SELECT policy.
#[tokio::test]
#[serial]
async fn tier_c_tables_have_rls_enabled_and_forced() {
    setup();
    let mut missing_enabled = Vec::new();
    let mut missing_forced = Vec::new();
    for table in EXPECTED_TIER_C_TABLES {
        if !rls_harness::table_rls_enabled(table).await {
            missing_enabled.push(*table);
        }
        if !rls_harness::table_force_rls(table).await {
            missing_forced.push(*table);
        }
    }
    assert!(
        missing_enabled.is_empty(),
        "Tier-C tables missing RLS ENABLE: {missing_enabled:?}"
    );
    assert!(
        missing_forced.is_empty(),
        "Tier-C tables missing FORCE ROW LEVEL SECURITY: {missing_forced:?}"
    );
}

#[tokio::test]
#[serial]
async fn tier_c_tables_have_named_policy_on_api_role() {
    setup();
    let attachments = rls_harness::policies_for_tables(EXPECTED_TIER_C_TABLES).await;
    for table in EXPECTED_TIER_C_TABLES {
        let policies = attachments
            .get(*table)
            .expect("policy catalog query must return an entry per table");
        let expected_name = format!("{table}_viewer");
        let matched = policies.iter().find(|p| p.name == expected_name);
        assert!(
            matched.is_some(),
            "Tier-C table public.{table} is missing policy {expected_name}; found {policies:?}"
        );
        let matched = matched.unwrap();
        assert!(
            matched.roles.iter().any(|r| r == "poziomki_api"),
            "Tier-C policy {expected_name} on public.{table} must target poziomki_api (targets: {:?})",
            matched.roles
        );
    }
}

/// The Tier-A migration installs three policy-support helpers in
/// schema `app`. Drop-regression canary.
#[tokio::test]
#[serial]
async fn tier_a_policy_helpers_exist() {
    setup();
    let configs = rls_harness::policy_helper_names().await;
    for name in EXPECTED_POLICY_HELPERS {
        assert!(
            configs.iter().any(|n| n == name),
            "policy helper app.{name} missing (found {configs:?})"
        );
    }
}
