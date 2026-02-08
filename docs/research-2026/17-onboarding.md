# Projektowanie onboardingu

## Zadania

1. **Marzec 2026**: Zmapować flow onboardingu i zidentyfikować punkty porzucenia (analityka)
2. **Kwiecień 2026**: Przeprowadzić 5-10 testów użyteczności z prawdziwymi studentami
3. **Kwiecień 2026**: Zaimplementować progresywne ujawnianie funkcji (maksymalnie 3-7 kroków)
4. **Maj 2026**: Dodać opcję „pomiń" z możliwością powrotu później
5. **Lipiec 2026**: Monitorować wskaźnik ukończenia onboardingu jako KPI (cel: 70%+)

---

## Benchmarki retencji dla aplikacji społecznościowych (2025)

Zanim zaprojektujemy onboarding, musimy znać realia branży.

**Retencja aplikacji społecznościowych:**

| Dzień | Retencja | Co to oznacza |
|-------|----------|---------------|
| D1 | 26-30% | 70-74% użytkowników nie wraca po pierwszym dniu |
| D7 | 9-13% | Tylko co dziesiąty zostaje po tygodniu |
| D30 | 3-7% | Po miesiącu zostaje mniej niż co dziesiąty |

**Dodatkowe statystyki:**
- 77% dziennych użytkowników porzuca aplikację w ciągu 3 dni
- iOS ma lepszą retencję niż Android (27% vs 24% D1)
- 25% porzuca aplikację po jednorazowym użyciu

**Cele dla Poziomek (realistyczne):**
- D1: 30%
- D7: 20%
- D30: 10%

Lepsze wyniki = product-market fit. Gorsze = powrót do badań użytkowników.

---

## Problem pustego ekranu (cold start)

Nowy użytkownik otwiera aplikację i widzi... nic. Brak dopasowań, brak wydarzeń, brak aktywności. To najpewniejszy sposób na utratę użytkownika.

**Dlaczego to się zdarza:**
- Za mało użytkowników w danej lokalizacji
- Brak treści startowej
- Algorytm nie ma danych do dopasowań

**Rozwiązania:**

| Problem | Rozwiązanie |
|---------|-------------|
| Brak dopasowań | Pokazać osoby z podobnymi zainteresowaniami (nawet luźne dopasowanie) |
| Brak wydarzeń | Wyświetlić nadchodzące wydarzenia na kampusie (z zewnętrznych źródeł) |
| Pusty feed | Pokazać „przykładowe" profile lub wydarzenia jako inspirację |

**Kluczowa zasada:**
Użytkownik musi zobaczyć wartość w ciągu pierwszych 30 sekund. Jedno dopasowanie wystarczy, żeby zrozumieć sens aplikacji.

---

## Zasada minimalnego wysiłku

Każdy dodatkowy krok w onboardingu obniża konwersję o około 20%. Minimalizm jest kluczowy.

**Minimum do pokazania wartości:**
1. Imię (lub pseudonim)
2. Zdjęcie (opcjonalne na start)
3. 2-3 zainteresowania (z listy)

To wystarczy, żeby pokazać pierwsze dopasowania.

**Reszta później:**
- Bio, opis, więcej zainteresowań — zbierane stopniowo
- Kontekstowe prośby: „Dodaj więcej zainteresowań, żebyśmy lepiej dopasowali"
- Gamifikacja: „Uzupełnij profil w 80% i odblokuj..."

**Parametry onboardingu:**
- Maksymalnie 3-7 kroków
- Czas ukończenia: poniżej 60 sekund
- Każdy krok = jasna wartość dla użytkownika

---

## Progresywne odkrywanie funkcji

Nie pokazuj wszystkiego naraz. Wprowadzaj funkcje, gdy stają się potrzebne.

**Przykłady progresywnego onboardingu:**

| Moment | Funkcja do wprowadzenia |
|--------|------------------------|
| Pierwsze dopasowanie | Jak wysłać wiadomość |
| Pierwsza wiadomość odebrana | Jak odpowiedzieć, reakcje |
| Tydzień użytkowania | Jak tworzyć wydarzenia |
| Po wzięciu udziału w wydarzeniu | Jak ocenić wydarzenie |

**Duolingo jako wzór:**
- Streaki, poziomy, nagrody wprowadzane stopniowo
- Nie wszystko od razu — budowanie nawyku
- D7 retention około 55% (vs średnia 13% dla edukacji)

---

## Prośby o uprawnienia — strategia

Użytkownicy są coraz bardziej ostrożni wobec prośb o uprawnienia. Źle zaprojektowane prośby = odmowa i utrata funkcjonalności.

**Zasady:**

1. **Pytaj tylko gdy potrzebne:**
   - Nie pytaj o lokalizację na starcie
   - Pytaj, gdy użytkownik chce zobaczyć „kto jest w pobliżu"

2. **Wyjaśnij dlaczego:**
   - „Żebyśmy mogli pokazać wydarzenia w Twojej okolicy"
   - „Żebyś wiedział, gdy ktoś Ci odpowie"

3. **Buduj zaufanie przed prośbą:**
   - Najpierw pokaż wartość, potem pytaj o uprawnienia
   - „Chcesz wiedzieć o nowych wiadomościach?" → push notification permission

---

## Społeczny dowód (social proof)

„X osób z Twojego wydziału już tu jest" — buduje FOMO i zaufanie jednocześnie.

**Zastosowania:**
- Ekran startowy: „Dołącz do 150 studentów z MIM UW"
- Onboarding: „5 osób z Twojego akademika już tu jest"
- Powiadomienia: „Ania, którą znasz, właśnie dołączyła"

**Uwaga:**
Social proof działa tylko przy masie krytycznej. Bez użytkowników lepiej tego nie pokazywać. Dlatego strategia launchu (koncentracja na jednym akademiku) jest kluczowa.

---

## Personalizacja od pierwszej chwili

Użytkownik powinien poczuć, że aplikacja „rozumie" go od początku.

**Praktyczne wskazówki:**
- Maksymalnie 5-7 kategorii zainteresowań do wyboru na start
- Mniej opcji = szybsza decyzja = wyższa konwersja
- Natychmiastowe dopasowania po wyborze zainteresowań

**Przykładowe kategorie startowe:**
- Kultura i sztuka
- Sport i aktywność
- Gry i technologia
- Nauka i kariera
- Podróże i jedzenie

Po onboardingu można dodać więcej szczegółów.

---

## Źródła

- AppsFlyer, *Mobile App Retention Benchmark Report 2024*
- Mixpanel, *Onboarding Funnel Analysis*
- Duolingo, *Retention Case Study*
- Nielsen Norman Group, *Mobile Onboarding Best Practices*
