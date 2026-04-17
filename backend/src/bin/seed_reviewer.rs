//! One-shot CLI to provision the Google Play reviewer test account plus a
//! small constellation of stub profiles and DMs. Safe to re-run: exits early
//! if the reviewer account already exists.
//!
//! Prereq: upload the stub photos to S3 first (keys listed in `PHOTOS`).

use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poziomki_backend::db::schema::{
    conversation_members, conversations, messages, profiles, uploads, user_settings, users,
};
use poziomki_backend::security;
use uuid::Uuid;

const REVIEWER_EMAIL: &str = "reviewer@poziomki.app";

struct StubSpec {
    email: &'static str,
    name: &'static str,
    bio: &'static str,
    program: &'static str,
    photo_key: &'static str,
}

const REVIEWER: StubSpec = StubSpec {
    email: REVIEWER_EMAIL,
    name: "Reviewer",
    bio: "Konto recenzenta Google Play.",
    program: "Informatyka",
    photo_key: "review/reviewer.webp",
};

const STUBS: &[StubSpec] = &[
    StubSpec {
        email: "stub01@poziomki.app",
        name: "Ola",
        bio: "Kocham planszówki i długie spacery po Krakowie.",
        program: "Psychologia",
        photo_key: "review/stub01.webp",
    },
    StubSpec {
        email: "stub02@poziomki.app",
        name: "Kuba",
        bio: "Bieganie o świcie, kawa o ósmej, kod do wieczora.",
        program: "Informatyka",
        photo_key: "review/stub02.webp",
    },
    StubSpec {
        email: "stub03@poziomki.app",
        name: "Marta",
        bio: "Studentka architektury, fanka kina i jazzu.",
        program: "Architektura",
        photo_key: "review/stub03.webp",
    },
    StubSpec {
        email: "stub04@poziomki.app",
        name: "Piotr",
        bio: "Wspinaczka, fotografia analogowa, dobre pierogi.",
        program: "Fizyka",
        photo_key: "review/stub04.webp",
    },
    StubSpec {
        email: "stub05@poziomki.app",
        name: "Zosia",
        bio: "Szukam kogoś do nauki języka włoskiego.",
        program: "Filologia włoska",
        photo_key: "review/stub05.webp",
    },
    StubSpec {
        email: "stub06@poziomki.app",
        name: "Jan",
        bio: "Muzyka elektroniczna, koty, góry.",
        program: "Matematyka",
        photo_key: "review/stub06.webp",
    },
    StubSpec {
        email: "stub07@poziomki.app",
        name: "Ania",
        bio: "Miłośniczka teatru i eksperymentalnej kuchni.",
        program: "Teatrologia",
        photo_key: "review/stub07.webp",
    },
    StubSpec {
        email: "stub08@poziomki.app",
        name: "Mateusz",
        bio: "Programowanie, rower, gry planszowe.",
        program: "Informatyka",
        photo_key: "review/stub08.webp",
    },
];

/// Scripted DM messages between reviewer and stubs 01..03. Even index =
/// from reviewer, odd = from stub.
const CHAT_SCRIPTS: &[(&str, &[&str])] = &[
    (
        "stub01@poziomki.app",
        &[
            "Cześć! Widziałam, że też lubisz planszówki :)",
            "Hej! Tak, ostatnio gram w Terraformację Marsa. A Ty?",
            "Uwielbiam! Może w sobotę zagramy?",
            "Jasne, może u mnie około 18?",
            "Pasuje. Przynieść coś słodkiego?",
            "Zawsze miło widziane!",
        ],
    ),
    (
        "stub02@poziomki.app",
        &[
            "Hej, biegasz po Plantach?",
            "Tak, codziennie rano o 6:30.",
            "Dołączę jutro, jeśli ok?",
            "Pewnie, spotkajmy się przy Barbakanie.",
            "Super, do jutra!",
        ],
    ),
    (
        "stub03@poziomki.app",
        &[
            "Widziałam Twój profil, też interesujesz się kinem?",
            "Tak, szczególnie Bergmana i Kieślowskiego.",
            "O, mam plakat Dekalogu w pokoju :)",
            "Kiedyś musisz mi pokazać!",
            "Jasne, może przy kawie?",
            "Chętnie. Znasz kawiarnię na Kazimierzu „Alchemia\"?",
            "Tak! Ulubione miejsce. Sobota 15:00?",
        ],
    ),
];

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
struct SeedUser {
    pid: Uuid,
    email: String,
    password: String,
    api_key: String,
    name: String,
    email_verified_at: chrono::DateTime<Utc>,
    is_review_stub: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_settings)]
struct SeedUserSettings {
    id: Uuid,
    user_id: i32,
    theme: String,
    language: String,
    notifications_enabled: bool,
    privacy_show_program: bool,
    privacy_discoverable: bool,
}

fn random_password() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*";
    let mut bytes = [0u8; 20];
    OsRng.fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| {
            let idx = (*b as usize) % CHARSET.len();
            CHARSET.get(idx).copied().unwrap_or(b'x') as char
        })
        .collect()
}

async fn user_exists(
    email: &str,
    conn: &mut poziomki_backend::db::DbConn,
) -> Result<Option<i32>, Box<dyn std::error::Error + Send + Sync>> {
    let row = users::table
        .filter(users::email.eq(email))
        .select(users::id)
        .first::<i32>(conn)
        .await
        .optional()?;
    Ok(row)
}

async fn create_seed_user(
    spec: &StubSpec,
    password_hash: &str,
    conn: &mut poziomki_backend::db::DbConn,
) -> Result<(i32, Uuid), Box<dyn std::error::Error + Send + Sync>> {
    let new_user = SeedUser {
        pid: Uuid::new_v4(),
        email: spec.email.to_string(),
        password: password_hash.to_string(),
        api_key: format!("rv-{}", Uuid::new_v4()),
        name: spec.name.to_string(),
        email_verified_at: Utc::now(),
        is_review_stub: true,
    };
    let user_id: i32 = diesel::insert_into(users::table)
        .values(&new_user)
        .returning(users::id)
        .get_result(conn)
        .await?;

    // user_settings
    let settings = SeedUserSettings {
        id: Uuid::new_v4(),
        user_id,
        theme: "system".to_string(),
        language: "pl".to_string(),
        notifications_enabled: true,
        privacy_show_program: true,
        privacy_discoverable: true,
    };
    diesel::insert_into(user_settings::table)
        .values(&settings)
        .execute(conn)
        .await?;

    // upload row for the profile photo
    let upload_id = Uuid::new_v4();
    let now = Utc::now();
    diesel::insert_into(uploads::table)
        .values((
            uploads::id.eq(upload_id),
            uploads::filename.eq(spec.photo_key),
            uploads::owner_id.eq::<Option<Uuid>>(None),
            uploads::context.eq("profile"),
            uploads::context_id.eq::<Option<String>>(None),
            uploads::mime_type.eq("image/webp"),
            uploads::deleted.eq(false),
            uploads::thumbhash.eq::<Option<Vec<u8>>>(None),
            uploads::has_variants.eq(false),
            uploads::created_at.eq(now),
            uploads::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;

    // profile
    let profile_id = Uuid::new_v4();
    diesel::insert_into(profiles::table)
        .values((
            profiles::id.eq(profile_id),
            profiles::user_id.eq(user_id),
            profiles::name.eq(spec.name),
            profiles::bio.eq::<Option<&str>>(Some(spec.bio)),
            profiles::profile_picture.eq::<Option<&str>>(Some(spec.photo_key)),
            profiles::images.eq::<Option<serde_json::Value>>(None),
            profiles::program.eq::<Option<&str>>(Some(spec.program)),
            profiles::gradient_start.eq::<Option<&str>>(None),
            profiles::gradient_end.eq::<Option<&str>>(None),
            profiles::created_at.eq(now),
            profiles::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;

    Ok((user_id, profile_id))
}

async fn seed_dm(
    reviewer_id: i32,
    stub_id: i32,
    script: &[&str],
    conn: &mut poziomki_backend::db::DbConn,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (low, high) = if reviewer_id < stub_id {
        (reviewer_id, stub_id)
    } else {
        (stub_id, reviewer_id)
    };
    let now = Utc::now();
    let conv_id = Uuid::new_v4();
    diesel::insert_into(conversations::table)
        .values((
            conversations::id.eq(conv_id),
            conversations::kind.eq("dm"),
            conversations::title.eq::<Option<&str>>(None),
            conversations::event_id.eq::<Option<Uuid>>(None),
            conversations::user_low_id.eq::<Option<i32>>(Some(low)),
            conversations::user_high_id.eq::<Option<i32>>(Some(high)),
            conversations::created_at.eq(now),
            conversations::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;

    for user in [reviewer_id, stub_id] {
        diesel::insert_into(conversation_members::table)
            .values((
                conversation_members::conversation_id.eq(conv_id),
                conversation_members::user_id.eq(user),
                conversation_members::joined_at.eq(now),
            ))
            .execute(conn)
            .await?;
    }

    // Space messages over the last ~7 days so the chat looks organic.
    let base = now - Duration::days(7);
    let step_minutes = if script.is_empty() {
        0
    } else {
        let total_minutes = 7 * 24 * 60_i64;
        total_minutes / i64::try_from(script.len()).unwrap_or(1).max(1)
    };
    for (idx, body) in script.iter().enumerate() {
        let sender = if idx % 2 == 0 { reviewer_id } else { stub_id };
        let idx_i64 = i64::try_from(idx).unwrap_or(0);
        let created_at = base + Duration::minutes(step_minutes * idx_i64);
        diesel::insert_into(messages::table)
            .values((
                messages::id.eq(Uuid::new_v4()),
                messages::conversation_id.eq(conv_id),
                messages::sender_id.eq(sender),
                messages::body.eq(*body),
                messages::kind.eq("text"),
                messages::reply_to_id.eq::<Option<Uuid>>(None),
                messages::client_id.eq::<Option<String>>(None),
                messages::created_at.eq(created_at),
            ))
            .execute(conn)
            .await?;
    }

    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set".to_string())?;
    poziomki_backend::db::init_pool(&database_url)?;

    let mut conn = poziomki_backend::db::conn().await?;

    if user_exists(REVIEWER_EMAIL, &mut conn).await?.is_some() {
        tracing::info!(
            "Reviewer account already exists ({}). Nothing to do.",
            REVIEWER_EMAIL
        );
        return Ok(());
    }

    // Reviewer
    let reviewer_password = random_password();
    let reviewer_hash = security::hash_password(&reviewer_password)
        .map_err(|e| format!("Argon2 hash failed: {e}"))?;
    let (reviewer_id, _reviewer_profile_id) =
        create_seed_user(&REVIEWER, &reviewer_hash, &mut conn).await?;
    tracing::info!("Created reviewer user id={}", reviewer_id);

    // Stubs
    let stub_hash = security::hash_password("stub-unused-password-0000")
        .map_err(|e| format!("Argon2 hash failed: {e}"))?;
    let mut stub_ids_by_email: std::collections::HashMap<&str, i32> =
        std::collections::HashMap::new();
    for stub in STUBS {
        let (id, _pid) = create_seed_user(stub, &stub_hash, &mut conn).await?;
        stub_ids_by_email.insert(stub.email, id);
        tracing::info!("Created stub user {} (id={})", stub.email, id);
    }

    // DMs (reviewer <-> stub01/02/03)
    for (email, script) in CHAT_SCRIPTS {
        if let Some(&stub_id) = stub_ids_by_email.get(*email) {
            seed_dm(reviewer_id, stub_id, script, &mut conn).await?;
            tracing::info!("Seeded DM reviewer <-> {}", email);
        }
    }

    tracing::info!("========================================");
    tracing::info!("REVIEWER EMAIL:    {}", REVIEWER_EMAIL);
    tracing::info!("REVIEWER PASSWORD: {}", reviewer_password);
    tracing::info!("Save it now; it will not be shown again.");
    tracing::info!("========================================");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let _ = dotenvy::dotenv();
    run().await
}
