# Metryki sukcesu

## Zadania

1. **Marzec 2026**: Zdefiniować 3-5 kluczowych wskaźników KPI przed udostępnieniem kodu (NIE czas w aplikacji!)
2. **Kwiecień 2026**: Wdrożyć analitykę (MAU, retencja D1/D7/D30, wskaźnik ukończenia onboardingu)
3. **Maj 2026**: Zaprojektować ankietę NPS dla beta użytkowników
4. **Lipiec 2026**: Dodać pytanie „Czy spotkałeś kogoś przez Poziomki?" — opcjonalne, nieinwazyjne
5. **Wrzesień 2026**: Pierwszy przegląd metryk i dostosowanie strategii

---

## Benchmarki branżowe 2025

Przed zdefiniowaniem własnych celów, warto znać realia branży aplikacji społecznościowych.

**Retencja aplikacji społecznościowych:**

| Metryka | Średnia branżowa | Cel dla Poziomek |
|---------|------------------|------------------|
| D1 retention | 26-30% | 30% |
| D7 retention | 9-13% | 20% |
| D30 retention | 3.9-7% | 10% |

**Inne benchmarki:**
- iOS ma lepszą retencję niż Android (27% vs 24% D1)
- Aplikacje gier i social mają najwyższą retencję (29% D1)
- Średnia dla wszystkich aplikacji: D1 28%, D7 18%, D30 8%

**Interpretacja wyników:**
- Lepiej niż benchmark = product-market fit, można skalować
- Gorzej niż benchmark = powrót do badań użytkowników

---

## North Star Metric — kluczowa metryka sukcesu

North Star Metric to jedna metryka, która najlepiej odzwierciedla wartość dostarczaną użytkownikom.

**Kandydat dla Poziomek:**
> Liczba spotkań offline zainicjowanych przez aplikację

**Problem:**
Spotkania offline są trudne do automatycznego zmierzenia. Nie mamy sensora „czy użytkownicy spotkali się w realu".

**Rozwiązania proxy:**

| Metoda | Opis |
|--------|------|
| Przycisk „spotkaliśmy się" | W czacie po wymianie wiadomości — obie strony potwierdzają |
| Ankieta po wydarzeniu | Krótka (1-3 pytania) następnego dnia po wydarzeniu |
| Deklarowane spotkania | „Czy umówiłeś spotkanie przez Poziomki?" — raz na miesiąc |

**Zasady pomiaru:**
- Opcjonalność: nie zmuszać do odpowiedzi
- Nieinwazyjność: jedno pytanie, nie formularz
- Kontekstowość: pytać w odpowiednim momencie

---

## Metryki zaangażowania (ilościowe)

**Podstawowe metryki skali:**

| Metryka | Opis | Częstotliwość pomiaru |
|---------|------|----------------------|
| MAU | Miesięczni aktywni użytkownicy | Miesięcznie |
| DAU | Dzienni aktywni użytkownicy | Tygodniowo |
| DAU/MAU ratio | Jak często użytkownicy wracają | Miesięcznie |

**Metryki retencji:**

| Metryka | Cel | Znaczenie |
|---------|-----|-----------|
| D1 retention | 30% | Pierwsze wrażenie |
| D7 retention | 20% | Budowanie nawyku |
| D30 retention | 10% | Długoterminowa wartość |

**Metryki onboardingu:**

| Metryka | Cel | Znaczenie |
|---------|-----|-----------|
| Completion rate | 70%+ | Czy onboarding nie jest za długi |
| Time to first match | <2 min | Czy szybko pokazujemy wartość |
| First message sent | 50% | Czy użytkownicy przechodzą do działania |

**Metryki aktywności:**

| Metryka | Co mierzy |
|---------|-----------|
| Message response rate | Aktywność w czatach |
| Event RSVP rate | Zainteresowanie wydarzeniami |
| RSVP → attendance | Konwersja deklaracji na spotkanie offline |

---

## Metryki jakościowe

**Net Promoter Score (NPS):**
- Pytanie: „W skali 0-10, jak bardzo poleciłbyś Poziomki znajomemu?"
- Wynik: -100 do +100
- Powyżej 0 = więcej promotorów niż krytyków
- Powyżej 50 = świetny wynik

**Match quality:**
- Ankieta po rozmowie: „Jak oceniasz tę rozmowę?"
- Skala 1-5 lub emotikony
- Opcjonalne, nieinwazyjne

**Event satisfaction:**
- Ankieta po wydarzeniu: „Jak oceniasz to wydarzenie?"
- „Czy poznałeś kogoś nowego?"
- Krótko — 1-3 pytania

---

## Czego NIE mierzyć (metryki sprzeczne z misją)

Niektóre metryki, standardowe dla platform społecznościowych, są sprzeczne z misją Poziomek.

**Metryki do unikania:**

| Metryka | Dlaczego nie |
|---------|--------------|
| Czas w aplikacji | Wysoki czas = porażka misji (cel to spotkanie offline) |
| Liczba scrolli | Zachęcanie do scrollowania = dark pattern |
| Message volume | Jakość ważniejsza niż ilość |
| „Viralność" | Growth hacking kosztem UX |

**Zasada:**
Jeśli optymalizacja metryki prowadzi do zachowań sprzecznych z misją, ta metryka nie powinna być KPI.

---

## Dashboard dla grantodawców

NLnet i inni grantodawcy wymagają raportowania wpływu projektu. Dashboard powinien być gotowy przed launchem.

**Metryki do raportowania:**

| Kategoria | Metryki |
|-----------|---------|
| Skala | MAU, liczba zarejestrowanych użytkowników, zasięg geograficzny |
| Zaangażowanie | Retencja D1/D7/D30, events created, messages sent |
| Jakość | NPS, match quality, event satisfaction |
| Wpływ | Zadeklarowane spotkania offline, ankiety jakościowe |

**Zasada transparentności:**
Publiczne metryki (zanonimizowane) budują zaufanie i pokazują, że projekt jest poważny. Rozważyć udostępnienie podstawowych statystyk na stronie projektu.

---

## Źródła

- AppsFlyer, *Mobile App Retention Benchmark Report 2024*
- Mixpanel, *Product Analytics Best Practices*
- Amplitude, *North Star Metric Framework*
- NLnet Foundation, *Impact Reporting Guidelines*
