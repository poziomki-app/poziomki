# Fediverse i protokół ActivityPub

## Zadania

1. **2026**: Monitorować integrację Threads i Tumblr z ActivityPub jako wskaźnik mainstream adoption
2. **2026**: Śledzić program NGI Fediversity NLnet jako potencjalne źródło finansowania
3. **2027+**: Rozważyć implementację ActivityPub po stabilizacji core aplikacji (v1.0+)
4. **2027+**: Przeanalizować kompromisy: otwartość vs kontrola nad moderacją przy federacji
5. **2027+**: Konsultacja techniczna: wykonalność federacji dla aplikacji mobile-first

---

## Czym jest Fediverse?

Fediverse (federacyjny wszechświat) to sieć zdecentralizowanych platform społecznościowych, które komunikują się ze sobą za pomocą otwartych protokołów. Użytkownik jednej platformy może śledzić i interagować z użytkownikami innych platform.

**Kluczowa koncepcja:**
Zamiast jednej firmy kontrolującej całą sieć (jak Facebook), Fediverse składa się z tysięcy niezależnych serwerów prowadzonych przez różne organizacje i osoby, które wymieniają się treściami.

---

## Statystyki Fediverse (2025-2026)

**Mastodon:**
- Ponad 8 mln użytkowników (wzrost z poniżej 1 mln przed 2022)
- 30% wzrost po aktualizacjach 2025 (cytowanie postów, narzędzia anty-harassment)
- Tysiące niezależnych serwerów (instancji)
- Tematyczne społeczności: dziennikarze, LGBTQ+, akademia, technologia

**Bluesky:**
- **40,2 mln użytkowników** (styczeń 2026, [Backlinko](https://backlinko.com/bluesky-statistics))
- 620% wzrost rok do roku (2024→2025)
- 62% użytkowników poniżej 34 lat — konkuruje o tę samą grupę demograficzną co Poziomki
- Używa własnego protokołu AT Protocol, nie ActivityPub

**Ruch sieciowy:**
Serwery Mastodona generują porównywalny ruch do Bluesky — użytkownicy śledzą bezpośrednio profile, nie odwiedzają stron internetowych.

---

## Protokoły federacyjne

**ActivityPub:**
- Standard W3C od 2018 roku
- Używany przez: Mastodon, PeerTube, PixelFed, Funkwhale
- Threads (Meta), WordPress, Ghost, Tumblr w trakcie integracji
- Mainstream momentum — wsparcie dużych graczy

**AT Protocol (Bluesky):**
- Alternatywa dla ActivityPub
- Priorytet: użyteczność bez poświęcania decentralizacji
- Bardziej elastyczna architektura niż ActivityPub
- Kontrolowany przez jedną firmę (Bluesky PBC)

**Ekosystem Fediverse:**
- PixelFed — alternatywa dla Instagramu
- PeerTube — alternatywa dla YouTube
- Funkwhale — alternatywa dla Spotify
- Lemmy — alternatywa dla Reddita

---

## NGI Fediversity — potencjalne finansowanie

NLnet prowadzi program NGI Fediversity finansujący projekty rozwijające technologie federacyjne.

**Cele programu:**
- Łatwe w użyciu usługi chmurowe z możliwością przenoszenia danych
- Rozwój standardów i narzędzi Fediverse
- Demokratyzacja infrastruktury społecznościowej

**Przykład:**
Mastodon otrzymał wsparcie z programów NLnet na rozwój funkcji i bezpieczeństwa.

**Dla Poziomek:**
Jeśli federacja stanie się kierunkiem rozwoju, NGI Fediversity może być źródłem finansowania implementacji ActivityPub.

---

## Federacja a Poziomki — kompromisy

**Argumenty za federacją:**

| Zaleta | Opis |
|--------|------|
| Interoperacyjność | Łączenie z użytkownikami innych platform Fediverse |
| Przenoszalność | Użytkownicy mogą przenieść dane do innej instancji |
| Zgodność z ekosystemem | Część rosnącego ruchu decentralizacji |
| Funding | Dostęp do programów jak NGI Fediversity |

**Argumenty przeciw federacji (na obecnym etapie):**

| Wyzwanie | Opis |
|----------|------|
| Moderacja | Trudniejsza kontrola treści przychodzących z zewnątrz |
| Architektura | Serwer federacyjny ≠ API mobilne — wymaga przebudowy |
| Złożoność UX | Federacja wprowadza koncepcje trudne dla zwykłego użytkownika |
| Over-engineering | Dla MVP studenckiego to zbyt ambitne |

---

## Rekomendacja dla Poziomek

**Na 2026 rok:**
Federacja to zbyt duże przedsięwzięcie dla początkowej fazy projektu. Poziomki są aplikacją local-first, skoncentrowaną na jednym kampusie — federacja z globalną siecią nie daje wartości na tym etapie.

**Co robić teraz:**
1. Monitorować rozwój ActivityPub i adopcję przez mainstream (Threads, Tumblr)
2. Śledzić NGI Fediversity jako potencjalne finansowanie na przyszłość
3. Projektować architekturę z myślą o przyszłej federacji (separacja concerns)

**Kiedy rozważyć federację:**
- Po stabilizacji core aplikacji (v1.0+)
- Gdy baza użytkowników przekroczy jeden kampus
- Gdy pojawi się realne zapotrzebowanie na łączność z innymi platformami
- Gdy zespół będzie miał zasoby na implementację i utrzymanie

---

## Źródła

- W3C, *ActivityPub Specification*
- Fediverse.observer, *Network Statistics*
- Bluesky, *AT Protocol Documentation*
- NLnet, *NGI Fediversity Programme*
- TechCrunch, *Threads ActivityPub Integration* (2024)
