# Jak kontrybować

## Wymagania

- [Rust](https://rustup.rs/) (stable) — backend
- [JDK 21](https://adoptium.net/) — aplikacja mobilna
- [PostgreSQL](https://www.postgresql.org/) — baza danych
- Opcjonalnie: [Nix](https://nixos.org/) — `nix develop` dostarczy cały toolchain za Ciebie
- Opcjonalnie: [Docker](https://www.docker.com/) — wygodny sposób na uruchomienie Postgres

## Przygotowanie

Skopiuj plik konfiguracji i uzupełnij wartości:

```sh
cp .env.example .env
```

Potrzebujesz co najmniej `DATABASE_URL` i `JWT_SECRET`. Przykładowe wartości znajdziesz w `.env.example`.

## Backend

```sh
cd backend
cargo run
```

Formatowanie i lintowanie:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Testy (wymagają działającego Postgres):

```sh
cargo test
```

Jeśli korzystasz z Nix, `nix fmt` uruchomi treefmt (rustfmt, ktfmt, shfmt i inne) dla całego repozytorium.

## Aplikacja mobilna

```sh
cd mobile
./gradlew :androidApp:assembleDebug
```

Lintowanie:

```sh
./gradlew detekt
./gradlew ktlintCheck
```

## Zgłaszanie issues

Użyj szablonów w zakładce [Issues](../../issues/new/choose):

- **Błąd** — opis problemu, kroki do odtworzenia, platforma
- **Propozycja** — opis funkcjonalności i motywacja

W razie wątpliwości — zgłoś, ustalimy.
