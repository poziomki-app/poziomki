//! CSV export of pre-launch (early-access) signups for store-testing rosters.
//!
//! Usage:
//!   cargo run --bin export-testers -- [--platform=android|ios|either|all]
//!                                     [--only-verified]
//!
//! Prints CSV to stdout with columns:
//!   email, name, platform_pref, email_verified, profile_completed, signed_up_at
//!
//! `--only-verified` filters to users who have completed OTP. Pipe through
//! `> android.csv` for direct import into Google Play Internal Testing or
//! TestFlight via the App Store Connect web UI.
//!
//! Connects via `DATABASE_URL` (same env var the migration runner uses),
//! NOT through the API pool — the operator running this CLI is expected
//! to have full read access to `public.users`.

#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::expect_used)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_const_for_fn)]

use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Nullable, Timestamptz, VarChar};

#[derive(Debug, QueryableByName)]
struct TesterRow {
    #[diesel(sql_type = VarChar)]
    email: String,
    #[diesel(sql_type = VarChar)]
    name: String,
    #[diesel(sql_type = Nullable<VarChar>)]
    platform_pref: Option<String>,
    #[diesel(sql_type = Bool)]
    email_verified: bool,
    #[diesel(sql_type = Bool)]
    profile_completed: bool,
    #[diesel(sql_type = Timestamptz)]
    signed_up_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformFilter {
    Android,
    Ios,
    Either,
    All,
}

impl PlatformFilter {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "android" => Ok(Self::Android),
            "ios" => Ok(Self::Ios),
            "either" => Ok(Self::Either),
            "all" | "any" | "" => Ok(Self::All),
            other => Err(format!("unknown --platform={other}")),
        }
    }

    fn sql_filter(self) -> &'static str {
        match self {
            // "either" users go on every roster, so include them with any
            // platform-specific filter.
            Self::Android => "AND platform_pref IN ('android', 'either')",
            Self::Ios => "AND platform_pref IN ('ios', 'either')",
            Self::Either => "AND platform_pref = 'either'",
            Self::All => "",
        }
    }
}

struct CliArgs {
    platform: PlatformFilter,
    only_verified: bool,
}

fn parse_args() -> Result<CliArgs, String> {
    let mut platform = PlatformFilter::All;
    let mut only_verified = false;

    for arg in std::env::args().skip(1) {
        if arg == "--only-verified" {
            only_verified = true;
        } else if let Some(value) = arg.strip_prefix("--platform=") {
            platform = PlatformFilter::parse(value)?;
        } else if arg == "--help" || arg == "-h" {
            print_usage();
            std::process::exit(0);
        } else {
            return Err(format!("unknown arg: {arg}"));
        }
    }

    Ok(CliArgs {
        platform,
        only_verified,
    })
}

fn print_usage() {
    eprintln!("export-testers — dump pre-launch signups as CSV");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  export-testers [--platform=android|ios|either|all] [--only-verified]");
    eprintln!();
    eprintln!("Env:");
    eprintln!("  DATABASE_URL  — Postgres connection string (required)");
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

fn main() -> Result<(), String> {
    let _ = dotenvy::dotenv();

    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n");
            print_usage();
            std::process::exit(2);
        }
    };

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set".to_string())?;

    let mut conn =
        diesel::pg::PgConnection::establish(&database_url).map_err(|e| format!("connect: {e}"))?;

    let verified_filter = if args.only_verified {
        "AND u.email_verified_at IS NOT NULL"
    } else {
        ""
    };

    let query = format!(
        "SELECT
            u.email,
            COALESCE(p.name, u.name) AS name,
            u.platform_pref,
            (u.email_verified_at IS NOT NULL) AS email_verified,
            (p.id IS NOT NULL) AS profile_completed,
            u.pre_launch_signed_up_at AS signed_up_at
         FROM public.users u
         LEFT JOIN public.profiles p ON p.user_id = u.id
         WHERE u.pre_launch_signed_up_at IS NOT NULL
           {verified}
           {platform}
         ORDER BY u.pre_launch_signed_up_at ASC",
        verified = verified_filter,
        platform = args.platform.sql_filter(),
    );

    let rows: Vec<TesterRow> = diesel::sql_query(query)
        .load(&mut conn)
        .map_err(|e| format!("query: {e}"))?;

    println!("email,name,platform_pref,email_verified,profile_completed,signed_up_at");
    for row in &rows {
        println!(
            "{},{},{},{},{},{}",
            csv_escape(&row.email),
            csv_escape(&row.name),
            csv_escape(row.platform_pref.as_deref().unwrap_or("")),
            row.email_verified,
            row.profile_completed,
            row.signed_up_at.to_rfc3339(),
        );
    }

    eprintln!("exported {} rows", rows.len());
    Ok(())
}
