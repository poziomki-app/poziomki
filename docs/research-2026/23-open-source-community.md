# Budowanie społeczności open-source

## Zadania

1. **Luty 2026**: Przygotować CONTRIBUTING.md, szablony issues i PR (zgodnie z ROADMAP open-source checklist)
2. **Marzec 2026**: Wypracować system etykiet (good first issue, help wanted, itp.)
3. **Kwiecień 2026**: Włączyć branch protection i reproducible builds
4. **Maj 2026**: Promocja projektu na r/opensource, Hacker News, dev.to — pierwsze zewnętrzne kontrybucje?
5. **2026**: Monitorować zaangażowanie kontrybutorów, czas odpowiedzi, zdrowie społeczności

---

## ROADMAP open-source checklist (przypomnienie)

Przed udostępnieniem kodu publicznie, należy przygotować:

- [ ] System etykietowania PR i issues
- [ ] Szablony PR i issues
- [ ] Reproducible builds APK
- [ ] CONTRIBUTING.md z instrukcjami
- [ ] Branch protection na main
- [ ] Poprawne wersjonowanie (semver)
- [ ] Licencja (AGPL-3.0)
- [ ] Stricte zdefiniowane środowisko

---

## Doświadczenie kontrybutora (contributor experience)

Pierwsze wrażenie decyduje o tym, czy potencjalny kontrybutor zostanie.

**Kluczowe elementy:**

| Element | Znaczenie |
|---------|-----------|
| README | Czytelne instrukcje instalacji i uruchomienia |
| CONTRIBUTING.md | Jasny workflow: jak zgłaszać issues, jak tworzyć PR |
| Good first issues | Proste zadania dla początkujących, dobrze opisane |
| Szybka odpowiedź | Pierwsza reakcja w ciągu 48h buduje zaangażowanie |

**Redukcja tarcia:**
Każdy dodatkowy krok w setup zmniejsza liczbę potencjalnych kontrybutorów.
- Idealnie: uruchomienie projektu jednym poleceniem
- Docker/devcontainer dla spójnego środowiska
- Dokumentacja wszystkich wymagań systemowych

---

## System etykiet (labels)

Etykiety pomagają organizować pracę i ułatwiają nowym osobom znalezienie odpowiednich zadań.

**Etykiety dla nowych kontrybutorów:**

| Etykieta | Znaczenie |
|----------|-----------|
| `good first issue` | Proste zadania z jasnym opisem — idealne na start |
| `help wanted` | Potrzebne wsparcie zewnętrzne |

**Etykiety typu:**

| Etykieta | Znaczenie |
|----------|-----------|
| `bug` | Błąd do naprawienia |
| `enhancement` | Nowa funkcja lub ulepszenie |
| `docs` | Dokumentacja |
| `refactor` | Refaktoryzacja kodu |

**Etykiety obszaru:**

| Etykieta | Znaczenie |
|----------|-----------|
| `mobile` | Aplikacja mobilna (Expo) |
| `api` | Backend (Elysia) |
| `design` | UI/UX |
| `infra` | Infrastruktura, CI/CD |

**Etykiety priorytetu:**

| Etykieta | Znaczenie |
|----------|-----------|
| `priority:high` | Pilne |
| `priority:medium` | Ważne, ale nie pilne |
| `priority:low` | Może poczekać |

---

## Kanały komunikacji

Społeczność potrzebuje miejsca do rozmów poza kodem.

**GitHub Discussions:**
- Zintegrowane z repozytorium
- Kategorie: Announcements, Q&A, Ideas, Show & Tell
- Dobre dla asynchronicznej komunikacji

**Matrix lub Discord:**
- Szybsza komunikacja synchroniczna
- Przydatne dla aktywnych kontrybutorów
- Wymaga moderacji

**Transparentność:**
- Publiczny roadmap (GitHub Projects lub ROADMAP.md)
- Log decyzji architektonicznych (ADR)
- Regularne aktualizacje (monthly update, changelog)

---

## Motywacja kontrybutorów

Ludzie kontrybuują do open-source z różnych powodów.

**Co przyciąga kontrybutorów:**

| Motywacja | Jak Poziomki to oferują |
|-----------|------------------------|
| Portfolio | Prawdziwy projekt z użytkownikami |
| Nauka | Nowoczesny stack (React Native, Elysia, Drizzle) |
| Misja | Rozwiązywanie problemu samotności studentów |
| Społeczność | Dołączenie do europejskiego projektu open-source |
| Kredyt | Uznanie w README, liście kontrybutorów |

**Jak budować społeczność:**
- Dziękować za każdą kontrybucję (nawet małą)
- Wymieniać kontrybutorów w changelog
- Dawać feedback szybko i konstruktywnie
- Zapraszać aktywnych kontrybutorów do głębszego zaangażowania

---

## Ryzyka i mitygacja

**Wypalenie maintainerów:**
- 3 studentów = ograniczona przepustowość
- Ustalić realistyczne oczekiwania: „Przeglądamy PR co tydzień", nie „natychmiast"
- Rotacja obowiązków w zespole

**Kontrola jakości:**
- Zewnętrzne PR wymagają review
- CI/CD wyłapuje podstawowe problemy (linting, testy, typy)
- Kultura code review — każdy PR przechodzi przez co najmniej jedną osobę

**Toksyczne kontrybucje:**
- Code of Conduct jasno określający zasady
- Szybka reakcja na naruszenia
- Moderacja dyskusji

---

## Metryki zdrowia społeczności

**Metryki ilościowe:**

| Metryka | Co mierzy |
|---------|-----------|
| Stars, forks | Zainteresowanie (vanity, ale sygnał) |
| Contributors count | Wielkość społeczności |
| PR merge time | Responsywność maintainerów |
| Issue response time | Jak szybko reagujemy |
| Contributor retention | Czy ludzie wracają |

**Metryki jakościowe:**
- Ton dyskusji (przyjazny vs toksyczny)
- Jakość kontrybucji (rośnie z czasem?)
- Feedback od kontrybutorów

---

## Źródła

- GitHub, *Open Source Guides* (opensource.guide)
- Nadia Eghbal, *Working in Public: The Making and Maintenance of Open Source*
- Linux Foundation, *Open Source Community Health Metrics*
- GitHub, *Community Standards Documentation*
