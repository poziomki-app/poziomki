# Działanie 04 — System rekomendacyjny dopasowujący użytkowników na podstawie zainteresowań

## Kontekst

System operuje na puli **90 kuratorowanych tagów zainteresowań** podzielonych na 12 kategorii (sport, muzyka, sztuka, technologia itp.). Użytkownicy wybierają minimum 3 tagi podczas onboardingu, bez górnego limitu. Zadaniem algorytmu jest uszeregowanie kandydatów według zgodności zainteresowań z profilem użytkownika.

## Przegląd rozważanych algorytmów

### 1. Indeks Jaccarda (implementacja początkowa)

**Wzór:** `|A ∩ B| / |A ∪ B|`

Stosunek części wspólnej do sumy zbiorów tagów obu użytkowników.

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Jaccard | Wynik |
|----------|------|---------|---------|-------|
| Osoba A (3 tagi) | 1 wspólny | 1/7 | 14 |
| Osoba B (10 tagów) | 4 wspólne | 4/11 | 36 |
| Osoba C (20 tagów) | 5 wspólnych | 5/20 | 25 |

**Problem:** Osoba C dzieli *wszystkie* zainteresowania użytkownika, ale uzyskuje niższy wynik niż osoba B, która dzieli tylko 4. Algorytm karze użytkowników posiadających wiele tagów — im więcej zainteresowań kandydat zaznaczył, tym większy mianownik (suma zbiorów) i tym niższy wynik. Prowadzi to do sytuacji, w której osoba o 1 wspólnym tagu może wyprzedzić osobę o 2 wspólnych tagach, jeśli ta pierwsza ma mniej tagów ogółem.

---

### 2. Podobieństwo cosinusowe

**Wzór:** `|A ∩ B| / √(|A| · |B|)`

Kąt między wektorami binarnymi tagów obu użytkowników.

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Cosinus | Wynik |
|----------|------|---------|---------|-------|
| Osoba A (3 tagi) | 1 wspólny | 1/√15 = 0.26 | 26 |
| Osoba B (10 tagów) | 4 wspólne | 4/√50 = 0.57 | 57 |
| Osoba C (20 tagów) | 5 wspólnych | 5/√100 = 0.50 | 50 |

**Problem:** Łagodniejszy niż Jaccard (pierwiastek zamiast sumy liniowej), ale nadal normalizuje po obu stronach. Kandydat z małą liczbą tagów wciąż zyskuje przewagę — 1 wspólny tag przy 2 tagach ogółem daje wyższy wynik niż 4 wspólne przy 20 tagach.

---

### 3. Współczynnik Dice'a (Sørensena)

**Wzór:** `2 · |A ∩ B| / (|A| + |B|)`

Średnia harmoniczna pokrycia obu zbiorów.

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Dice | Wynik |
|----------|------|---------|------|-------|
| Osoba A (3 tagi) | 1 wspólny | 2/8 = 0.25 | 25 |
| Osoba B (10 tagów) | 4 wspólne | 8/15 = 0.53 | 53 |
| Osoba C (20 tagów) | 5 wspólnych | 10/25 = 0.40 | 40 |

**Problem:** Matematycznie równoważny podejściu F1-score. Podobne wady jak cosinus — uśrednia pokrycie obu stron, więc duża liczba tagów kandydata obniża wynik nawet przy pełnym dopasowaniu.

---

### 4. Współczynnik Overlap (Szymkiewicza-Simpsona)

**Wzór:** `|A ∩ B| / min(|A|, |B|)`

Pokrycie mniejszego zbioru.

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Overlap | Wynik |
|----------|------|---------|---------|-------|
| Osoba A (3 tagi) | 1 wspólny | 1/3 = 0.33 | 33 |
| Osoba B (10 tagów) | 4 wspólne | 4/5 = 0.80 | 80 |
| Osoba C (20 tagów) | 5 wspólnych | 5/5 = 1.00 | 100 |

**Problem:** Odwrotna wada — osoba z 1 tagiem pasującym do 1 z naszych uzyskuje wynik 1/1 = 100, taki sam jak pełne dopasowanie 5/5. Nie rozróżnia głębokości dopasowania.

---

### 5. Surowe liczenie wspólnych tagów

**Wzór:** `|A ∩ B|`

Prosta liczba wspólnych zainteresowań.

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Wynik |
|----------|------|---------|-------|
| Osoba A (3 tagi) | 1 wspólny | 1 |
| Osoba B (10 tagów) | 4 wspólne | 4 |
| Osoba C (20 tagów) | 5 wspólnych | 5 |

**Zaleta:** Prosty, czytelny, więcej wspólnych tagów = wyższy wynik.

**Problem:** Użytkownik, który zaznaczy 20 z 90 tagów, statystycznie będzie mieć więcej dopasowań z każdym kandydatem niż użytkownik z 3 tagami. Oznacza to, że zaznaczenie jak największej liczby tagów staje się optymalną strategią — „tag hoarding" poprawia widoczność w rekomendacjach bez odzwierciedlenia prawdziwych zainteresowań.

---

### 6. Recall — pokrycie własnych zainteresowań (wybrane rozwiązanie)

**Wzór:** `|A ∩ B| / |A| · 100 + bonus_kierunek`

Jaki procent *moich* zainteresowań podziela kandydat?

**Przykład** (użytkownik ma 5 tagów):

| Kandydat | Tagi | Wspólne | Recall | Wynik |
|----------|------|---------|--------|-------|
| Osoba A (3 tagi) | 1 wspólny | 1/5 | 20 |
| Osoba B (10 tagów) | 4 wspólne | 4/5 | 80 |
| Osoba C (20 tagów) | 5 wspólnych | 5/5 | 100 |
| Osoba D (20 tagów) | 3 wspólne | 3/5 | 60 |

**Zalety:**
- Więcej wspólnych tagów = wyższy wynik, zawsze
- Liczba tagów kandydata nie wpływa na wynik — liczy się tylko to, ile *moich* zainteresowań podziela
- Naturalnie penalizuje „tag hoarding": zaznaczenie 20 tagów nie pomaga, bo mianownik (moje tagi) rośnie proporcjonalnie — wynik 5/20 = 25 zamiast 5/5 = 100
- Skala 0–100 jest czytelna: 60 oznacza „60% moich zainteresowań jest wspólnych"

**Ochrona przed gamingiem:**

| Moje tagi | Kandydat (3 tagi: 1,2,3) | Wspólne | Wynik |
|-----------|--------------------------|---------|-------|
| 3 | 3 | 100 |
| 5 | 3 | 60 |
| 10 | 3 | 30 |
| 20 | 3 | 15 |
| 30 | 3 | 10 |

Zaznaczanie coraz większej liczby tagów obniża wyniki wszystkich rekomendacji — użytkownik jest motywowany do wybierania tylko tagów, które go naprawdę interesują.

## Bonus za kierunek studiów

Do wyniku recall dodawany jest bonus **+5 punktów** za ten sam kierunek studiów. Przy minimalnej różnicy jednego tagu (20 punktów dla użytkownika z 5 tagami) bonus nie jest w stanie odwrócić kolejności — pełni rolę rozstrzygnięcia remisów.

| Kandydat | Wspólne tagi | Kierunek | Wynik |
|----------|-------------|----------|-------|
| 2 wspólne, inny kierunek | 2/5 | — | 40 |
| 1 wspólny, ten sam kierunek | 1/5 | +5 | 25 |

## Podsumowanie porównania

| Algorytm | Wzór | Karze dużo tagów? | Gaming? | Wybrany? |
|----------|------|-------------------|---------|----------|
| Jaccard | ∩/∪ | Tak (silnie) | Nie | Nie |
| Cosinus | ∩/√(a·b) | Tak (umiarkowanie) | Nie | Nie |
| Dice | 2∩/(a+b) | Tak (umiarkowanie) | Nie | Nie |
| Overlap | ∩/min(a,b) | Nie | Nie | Nie* |
| Surowe liczenie | ∩ | Nie | Tak | Nie |
| **Recall** | **∩/moje** | **Nie** | **Nie** | **Tak** |

*\* Overlap nie rozróżnia 1/1 od 5/5 — brak gradacji przy małych zbiorach.*
