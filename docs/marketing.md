# Materiały sklepowe poziomek

## Źródła ekranów

Android: rzeczywiste przechwycenia lokalnego buildu 0.22.5 z aktualnego kodu oraz fikcyjnej bazy demonstracyjnej. Baza zawiera 12 profili, 6 wydarzeń i 10 rozmów. Zrzuty nie zawierają kont prawdziwych użytkowników.

iPhone i iPad: oryginalne zrzuty pobrane 26.10.2026 r. z publicznego listingu App Store wersji 0.22.6. Nie zastępujemy ich Androidem stylizowanym na iOS. Letterboxing iPada odzwierciedla natywny materiał źródłowy.

Logotypy FE, RP, UE i Uczelni Przyszłości pochodzą z oryginalnego pliku `docs/assets/funding.png` (5500 × 710 px). Logo NCBR pochodzi z oficjalnego pliku wektorowego, wskazanego w `marketing/assets/FUNDING-SOURCES.txt`. Złożenie znajduje się w `docs/assets/funding-ue-up-ncbr.svg`. Pasek pozostaje na białym tle, bez zmiany proporcji i kolorów znaków.

## Ponowne generowanie

Wymagane są Node.js 22 lub nowszy oraz `agent-browser` z działającą przeglądarką. Uruchom w katalogu repozytorium:

```sh
node marketing/render-showcase.mjs all
node scripts/google-marketing.mjs validate
```

Renderer uruchamia serwer wyłącznie na loopback, eksportuje PNG bez kanału alfa i zamyka własną sesję przeglądarki. Zaktualizuj pliki w `source/` przed eksportem nowej wersji interfejsu.

## Google Play bez panelu WWW

Workflow korzysta z istniejącego sekretu `PLAY_SERVICE_ACCOUNT_JSON`:

```sh
gh workflow run android-release.yml --ref NAZWA_GALEZI -f marketing=true
```

Nie buduje ani nie przesyła APK/AAB. Zapisuje kopię opisów i oryginalnych grafik, wymienia polski listing w jednej transakcji i zatwierdza zmianę. Jeśli operacja się nie powiedzie, usuwa niezakończoną transakcję. Kopia jest dostępna jako artefakt `google-marketing-backup` przez 7 dni. Widoczność zmian zależy od kontroli i publikacji w Google Play.

Lokalnie można ustawić `GOOGLE_PLAY_SERVICE_ACCOUNT_FILE` i `STORE_OUTPUT_DIR`, a następnie uruchomić `node scripts/google-marketing.mjs upload`. Katalog kopii powinien znajdować się poza repozytorium, np. w `~/.cache/poziomki-store/`.

## App Store bez panelu WWW

Odczyt metadanych, bez buildu i zmian w sklepie:

```sh
gh workflow run ios-release.yml --ref NAZWA_GALEZI -f marketing=true
```

Przesłanie opisów i zrzutów do wskazanej wersji roboczej:

```sh
gh workflow run ios-release.yml --ref NAZWA_GALEZI \
  -f marketing=true -f marketing_upload=true -f app_store_version=0.22.7
```

Wybierz numer wersji przeznaczonej do następnego wydania. Proces nie przesyła IPA, nie wysyła aplikacji do review i nie publikuje wydania. Sklep wymaga aktywnych umów właściciela konta; błąd `REQUIRED_AGREEMENTS_MISSING_OR_EXPIRED` trzeba rozwiązać w App Store Connect przed użyciem API.
