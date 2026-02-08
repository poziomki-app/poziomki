# Ryzyka i wyzwania do przemyślenia

## Zadania

1. **Luty 2026**: Opracować plan finansowania na 12 miesięcy (NLnet jako główne źródło)
2. **Marzec 2026**: Napisać Community Guidelines i procedury reagowania przed udostępnieniem kodu
3. **Marzec 2026**: Przygotować procedurę breach notification (72h do UODO)
4. **Kwiecień 2026**: Zaplanować strategię launchu: jeden akademik → masa krytyczna
5. **2026**: Zdefiniować exit strategy (open-source = projekt może żyć bez nas)

---

## Ryzyko 1: Wypalenie zespołu (KRYTYCZNE)

Wypalenie to największe zagrożenie dla projektu prowadzonego przez studentów. Statystyki dotyczące założycieli startupów są alarmujące.

**Statystyki burnout (2024-2025):**
- 54% założycieli doświadczyło wypalenia w ostatnim roku
- 72% kiedykolwiek w trakcie prowadzenia projektu
- 67% pracuje ponad 50 godzin tygodniowo
- 5% startupów upada bezpośrednio z powodu wypalenia założycieli
- 83% założycieli zgłasza wysoki poziom stresu
- 54% cierpi na bezsenność

**Dlaczego to dotyczy Poziomek:**
Zespół składa się z 3 studentów łączących projekt ze studiami. Nawet jeśli to „tylko" side-project, te same mechanizmy prowadzą do wypalenia.

**Strategie mitygacji:**
- Realistyczne oczekiwania: 5-15 godzin tygodniowo na osobę, nie więcej
- Jasny podział obowiązków: każdy obszar ma właściciela
- Wyraźne granice między nauką a projektem
- Mentality side-project: studia są priorytetem
- Regularne check-iny o samopoczuciu w zespole

Szczegółowy dokument: [25-team-sustainability.md](25-team-sustainability.md)

---

## Ryzyko 2: Brak zrównoważonego finansowania

Projekty open-source często upadają z braku środków na utrzymanie.

**Realia budżetowe innych projektów:**
- Signal Foundation: około 50 mln dolarów rocznie (nieosiągalne bez multimiliardera)
- Mastodon: cel 5 mln euro rocznie (wymaga tysięcy użytkowników i klientów instytucjonalnych)

**Realistyczny budżet dla Poziomek (2026):**
- Grant NLnet: 5-50 tys. euro (jednorazowo)
- Patronite: kilkadziesiąt-kilkaset zł miesięcznie
- Potencjalnie Fundusze Norweskie: konkurs Q1 2026

**Strategia mitygacji:**
- Plan finansowania na 12 miesięcy po zakończeniu grantu
- Dywersyfikacja źródeł (nie polegać na jednym grancie)
- Niskie koszty operacyjne (wolontariat, darmowe narzędzia)
- Exit plan: projekt może żyć jako community-driven bez założycieli

---

## Ryzyko 3: Naruszenie RODO/GDPR

Nieprzestrzeganie przepisów o ochronie danych może mieć poważne konsekwencje.

**Potencjalne kary:**
- Maksymalnie: 20 mln euro lub 4% rocznego obrotu
- Toyota Bank Polska: 78 tys. zł za opóźnione zgłoszenie naruszenia
- Morele.net: 660 tys. euro za wyciek danych klientów

**Kluczowe wymagania:**
- Breach notification: 72 godziny na zgłoszenie do UODO
- Dokumentacja przetwarzania danych od dnia 1
- Polityka prywatności w przystępnym języku
- Prawo do usunięcia danych i ich eksportu

**Strategia mitygacji:**
- Data protection by design od pierwszej linii kodu
- Przygotowana procedura breach notification
- Rejestr czynności przetwarzania
- Minimalizacja danych: zbierać tylko to, co niezbędne

---

## Ryzyko 4: Brak masy krytycznej użytkowników

Aplikacja społecznościowa bez użytkowników jest bezwartościowa — klasyczny problem „kurczaka i jajka".

**Statystyki porzucania aplikacji:**
- 23-25% użytkowników porzuca aplikację po jednorazowym użyciu
- 62% używa aplikacji mniej niż 11 razy przed porzuceniem

**Lekcja z Facebooka:**
- Połowa Harvardu w miesiąc
- Klucz: koncentracja na jednym kampusie przed ekspansją
- Ekskluzywność (tylko studenci z .edu) budowała poczucie społeczności

**Strategia mitygacji:**
- Launch z ambasadorami w konkretnym miejscu (jeden akademik lub wydział)
- Nie rozpraszać się: masa krytyczna w jednym miejscu, potem ekspansja
- Timing: Welcome Week (wrzesień) — studenci najbardziej otwarci na nowe kontakty
- Wartość od pierwszego dnia: nawet 50 aktywnych użytkowników w jednym akademiku tworzy wartość

---

## Ryzyko 5: Problemy z moderacją treści

Jeden przypadek nękania może pochłonąć godziny pracy zespołu i zniszczyć reputację projektu.

**Precedens: UNC (2024):**
System uczelni University of North Carolina zakazał aplikacji Yik Yak, Sidechat, Fizz i Whisper z powodu cyberprzemocy. Wspólny mianownik: anonimowość.

**Wyzwania:**
- Moderacja wymaga ciągłej dostępności
- Trudne decyzje (co jest dopuszczalne, a co nie)
- Potencjalne konsekwencje prawne (DSA, odpowiedzialność platformy)
- Uczelnie mogą zablokować aplikację jeśli stanie się źródłem problemów

**AI pomaga, ale nie zastępuje ludzi:**
- Automatyczna moderacja przetwarza treści 1000x szybciej
- Ale słabo radzi sobie z sarkazmem, kontekstem kulturowym, subtelnym nękaniem

**Strategia mitygacji:**
- **Brak anonimowości** — weryfikacja emailem uczelnianym, konta przypisane do osób
- Jasne Community Guidelines gotowe przed launchem
- Prosty workflow zgłoszeń z priorytetami
- Hybrydowe podejście: AI + human review
- Zero tolerancji dla oczywistych naruszeń (szybka reakcja buduje zaufanie)
- Współpraca z administracją uczelni (nie antagonizowanie)

---

## Ryzyko 6: Scope creep — pełzające wymagania

Dodawanie funkcji „na później" bez kończenia obecnych to częsta przyczyna wypalenia i opóźnień.

**Objawy:**
- Lista „nice to have" rośnie szybciej niż lista ukończonych zadań
- Każda funkcja wymaga „jeszcze jednej rzeczy" przed release
- Zespół pracuje nad wieloma rzeczami naraz, żadna nie jest gotowa

**Przykłady scope creep dla Poziomek:**
- „Dodajmy jeszcze ActivityPub" (za wcześnie)
- „Potrzebujemy własnego systemu analityki" (użyj gotowego)
- „Napiszmy dokumentację do wszystkiego" (pisz tylko to, co niezbędne)

**Strategia mitygacji:**
- Jasno zdefiniowany MVP na każdy kwartał
- „Not now" ≠ „never" — parking lot dla pomysłów
- Jeden sprint = jedna funkcja do końca
- Regularny przegląd: „czy to jest niezbędne do launchu?"

---

## Ryzyko 7: Brak ciągłości projektu (bus factor)

Co się stanie, jeśli kluczowa osoba odejdzie?

**Typowe problemy:**
- Wiedza tylko w głowach założycieli
- Brak dokumentacji decyzji architektonicznych
- Kod zrozumiały tylko dla autora

**Strategia mitygacji — zaleta open-source:**
- Kod publicznie dostępny
- Społeczność może kontynuować rozwój (fork)
- CONTRIBUTING.md i dokumentacja = plan sukcesji
- Licencja AGPL zapewnia, że forki pozostaną otwarte

**Exit strategy:**
Jeśli zespół nie może kontynuować:
1. Ogłoszenie sunsetting z terminem
2. Archiwizacja repozytorium (nie usuwanie)
3. Przekazanie społeczności lub innemu zespołowi
4. Honorowe zakończenie, nie porażka

---

## Źródła

- Startup Snapshot, *Founder Mental Health Report 2024*
- AppsFlyer, *Mobile App Retention Report 2024*
- UODO, *Decyzje i kary za naruszenie RODO*
- Nadia Eghbal, *Working in Public: The Making and Maintenance of Open Source*
