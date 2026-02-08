# Konta deweloperskie w sklepach aplikacji

## Zadania

1. **Marzec 2026**: Złożyć wniosek o numer D-U-N-S przez email do Dun & Bradstreet po uzyskaniu REGON (10 dni roboczych, bezpłatny)
2. **Marzec 2026**: Przygotować repozytorium do submisji F-Droid (metadata w formacie fastlane, tagi dla releasów)
3. **Kwiecień 2026**: Zarejestrować konto Google Play Console ($25) po synchronizacji D-U-N-S z bazą Google
4. **Kwiecień 2026**: Złożyć Request for Packaging na GitLab F-Droid (recenzja trwa około miesiąca)
5. **Maj 2026**: Zarejestrować konto Apple Developer ($99/rok — Polska nie kwalifikuje się do zwolnienia z opłaty)

---

## Numer D-U-N-S — fundament rejestracji

D-U-N-S (Data Universal Numbering System) to dziewięciocyfrowy identyfikator biznesowy wydawany przez Dun & Bradstreet. Jest wymagany zarówno przez Google, jak i Apple do rejestracji konta deweloperskiego jako organizacja.

**Jak uzyskać numer D-U-N-S w Polsce:**

1. Po otrzymaniu numeru REGON wyślij email na adres: `dnbeu-dunshelp.pl@dnb.com`
2. W treści podaj:
   - Pełną nazwę organizacji (dokładnie jak w REGON)
   - Adres siedziby
   - Numer REGON
   - Dane kontaktowe
3. Czas oczekiwania: około 10 dni roboczych
4. Koszt: bezpłatny w Polsce

**Ważne:** Nazwa organizacji w profilu D-U-N-S musi dokładnie odpowiadać nazwie w dokumentach rejestracyjnych. Niezgodność nazwy spowoduje odrzucenie przez Google lub Apple. Po utworzeniu profilu D-U-N-S, synchronizacja z bazami Google i Apple trwa do 48 godzin.

---

## Google Play Console

**Sprawdzone dane o rejestracji:**

| Informacja | Wartość |
|------------|---------|
| Opłata rejestracyjna | $25 (jednorazowa, około 100 zł) |
| Metody płatności | Karta kredytowa lub debetowa (nie prepaid) |
| Czas weryfikacji organizacji | 5-10 dni roboczych |
| Wymagany numer D-U-N-S | Tak, dla konta organizacji |

**Proces rejestracji konta organizacji:**

1. Utwórz konto Google (lub użyj istniejącego)
2. Załóż Business Google Payments Profile z danymi stowarzyszenia
3. Zarejestruj się w Google Play Console jako organizacja
4. Podaj numer D-U-N-S (musi być zsynchronizowany z bazą Google)
5. Prześlij dokumenty weryfikacyjne (statut/regulamin, dane przedstawiciela)
6. Opłać jednorazową opłatę $25
7. Poczekaj na weryfikację (5-10 dni)

**Zmiany w weryfikacji deweloperów (2024-2026):**

Google zaostrza wymagania weryfikacyjne dla deweloperów. Od 2024 roku wymaga:
- Weryfikacji tożsamości osoby rejestrującej (dokument tożsamości)
- Podania adresu fizycznego organizacji
- Ujawnienia identyfikatorów pakietów aplikacji

F-Droid ostrzega, że te zmiany mogą utrudnić niezależną dystrybucję aplikacji open-source. Temat był dyskutowany na FOSDEM 2025 i prawdopodobnie będzie kontynuowany w 2026.

---

## Apple Developer Program

**Sprawdzone dane:**

| Informacja | Wartość |
|------------|---------|
| Opłata roczna | $99 (około 400 zł) |
| Wymagany numer D-U-N-S | Tak |
| Wymagana strona internetowa | Tak, z domeną powiązaną z organizacją |
| Czas weryfikacji | 1-4 tygodnie |

**Wymagania dla organizacji:**

- Działająca strona internetowa z domeną odpowiadającą nazwie organizacji
- Osoba rejestrująca musi mieć prawne upoważnienie do reprezentowania organizacji
- Dokumenty potwierdzające status prawny organizacji

**Zwolnienie z opłaty dla non-profit — niedostępne w Polsce:**

Apple oferuje program fee waiver dla organizacji non-profit, ale **Polska nie kwalifikuje się do tego programu**. Lista krajów uprawnionych do zwolnienia (stan na 2026):
- Australia, Brazylia, Kanada, Chiny, Francja, Niemcy, Izrael, Włochy, Japonia, Meksyk, Korea Południowa, Wielka Brytania, USA

**Konsekwencja dla budżetu:**
- Należy zaplanować $99/rok (około 400 zł) na Apple Developer Program
- Opłata jest wymagana do publikacji na App Store i TestFlight
- Android (Google Play + F-Droid) pozostaje priorytetem ze względu na niższe koszty i większy zasięg w społeczności open-source

**Źródło:** [Apple Developer Fee Waivers](https://developer.apple.com/help/account/membership/fee-waivers/)

---

## F-Droid — sklep dla wolnego oprogramowania

F-Droid to niezależny sklep z aplikacjami dla Androida, zawierający wyłącznie wolne oprogramowanie (FOSS). Publikacja w F-Droid jest bezpłatna i nie wymaga zakładania konta.

**Wymagania dla aplikacji:**

- Kod źródłowy musi być w pełni otwarty (licencja FOSS)
- Wszystkie zależności muszą być również open-source
- Aplikacja nie może zawierać trackerów ani zamkniętych komponentów
- Metadane muszą być w formacie fastlane lub zgodnym z F-Droid

**Proces publikacji krok po kroku:**

1. **Przygotowanie repozytorium:**
   - Dodaj katalog `fastlane/metadata/android/` z opisami w różnych językach
   - Dodaj zrzuty ekranu i ikonę
   - Upewnij się, że wszystkie zależności są FOSS
   - Taguj release'y w git (np. `v0.1.0`)

2. **Zgłoszenie aplikacji:**
   - Sforkuj repozytorium `fdroiddata` na GitLab
   - Utwórz issue typu „Request for Packaging" opisujący aplikację
   - Po akceptacji przygotuj merge request „New app: Poziomki"

3. **Recenzja i publikacja:**
   - Zespół F-Droid sprawdza kod i metadane
   - F-Droid buduje aplikację ze źródeł (nie z dostarczonego APK)
   - Czas recenzji: około miesiąca
   - Po zatwierdzeniu aplikacja pojawia się w katalogu

**Reproducible builds:**

F-Droid preferuje reproducible builds — możliwość odtworzenia identycznego APK z kodu źródłowego. Nie jest to wymagane, ale zalecane dla zwiększenia zaufania użytkowników.

---

## Rekomendowany harmonogram

| Miesiąc | Działanie |
|---------|-----------|
| Luty 2026 | Rejestracja stowarzyszenia, uzyskanie REGON |
| Marzec 2026 | Wniosek o D-U-N-S, przygotowanie metadanych F-Droid |
| Kwiecień 2026 | Zgłoszenie do F-Droid, rejestracja Google Play |
| Maj 2026 | Rejestracja Apple Developer ($99/rok) |
| Wrzesień 2026 | Publikacja w Google Play i F-Droid na Welcome Week |

**Priorytet:** Android (Google Play + F-Droid) przed iOS. Społeczność open-source korzysta głównie z Androida, a F-Droid nie wymaga opłat.

---

## Źródła

- [support.google.com/googleplay/android-developer](https://support.google.com/googleplay/android-developer/answer/6112435) — rejestracja konta deweloperskiego
- [developer.apple.com/programs](https://developer.apple.com/programs/) — Apple Developer Program
- [f-droid.org/docs](https://f-droid.org/docs/) — dokumentacja F-Droid
- [dnb.com/duns-number](https://www.dnb.com/duns-number.html) — informacje o D-U-N-S
