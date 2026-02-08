# Dostępność i WCAG — wymaganie NLnet

## Zadania

1. **Q2 2026 (kwiecień-maj)**: Przeprowadzić podstawowy audyt WCAG 2.1 AA (automatyczne narzędzia + manualny przegląd)
2. **Q3 2026 (lipiec-sierpień)**: Naprawić krytyczne problemy wykryte w audycie (kontrast, touch targets, screen reader)
3. **Q3 2026**: Dodać ustawienia dostępności w aplikacji (rozmiar czcionki, wysoki kontrast, redukcja ruchu)
4. **Q3-Q4 2026**: Przetestować z użytkownikami z niepełnosprawnościami (2-3 osoby)
5. **2026**: Dokumentować dostępność jako cechę produktu, nie dodatek

> **Uwaga:** Pełna zgodność WCAG 2.1 AA wymagana dla wniosku NLnet (Q4 2026).

---

## Dlaczego dostępność jest krytyczna

**Wymóg NLnet:**
Wszystkie projekty finansowane przez NLnet muszą spełniać standardy dostępności WCAG. Brak zgodności = brak grantu.

**Statystyki:**
- Ponad 15% światowej populacji żyje z jakąś formą niepełnosprawności (WHO)
- 73% użytkowników z niepełnosprawnościami opuszcza strony i aplikacje trudne w nawigacji
- Inclusive design zwiększa zaangażowanie wszystkich użytkowników o 35%

**Nie tylko dla osób z niepełnosprawnościami:**
Dostępność poprawia UX dla wszystkich:
- Lepsze kontrasty = łatwiejsza obsługa w słońcu
- Większe touch targets = wygodniejsza obsługa jedną ręką
- Czytelne etykiety = szybsze zrozumienie interfejsu

---

## WCAG 2.1 AA — minimalne wymagania

WCAG (Web Content Accessibility Guidelines) to międzynarodowy standard dostępności. Poziom AA to standard wymagany przez NLnet.

### Perceivable (Postrzegalność)

| Wymaganie | Opis | Wartość |
|-----------|------|---------|
| Kontrast tekstu | Stosunek kontrastu tekstu do tła | Min 4.5:1 (3:1 dla dużego tekstu) |
| Tekst alternatywny | Opis obrazów dla screen readerów | Wszystkie obrazy informacyjne |
| Napisy | Dla treści audio/wideo | Wymagane |

### Operable (Funkcjonalność)

| Wymaganie | Opis | Wartość |
|-----------|------|---------|
| Nawigacja klawiaturą | Dostęp do wszystkich funkcji bez myszy | 100% funkcji |
| Touch targets | Rozmiar elementów dotykalnych | Min 44x44 pikseli |
| Czas na akcje | Wystarczający czas na reakcję | Możliwość wydłużenia |
| Miganie | Unikanie treści mogących wywołać napady | Max 3 błyski/s |

**Uwaga:** 72% problemów z dostępnością mobilną to za małe przyciski (touch targets).

### Understandable (Zrozumiałość)

| Wymaganie | Opis |
|-----------|------|
| Spójna nawigacja | Ten sam układ na wszystkich ekranach |
| Identyfikacja błędów | Jasne komunikaty o błędach z sugestiami |
| Etykiety formularzy | Każde pole ma opisową etykietę |

### Robust (Solidność)

| Wymaganie | Opis |
|-----------|------|
| Screen reader | Kompatybilność z VoiceOver/TalkBack |
| Poprawna struktura | Semantyczny HTML/markup |

---

## Dostępność w React Native / Expo

React Native i Expo zapewniają narzędzia do budowania dostępnych aplikacji.

**Kluczowe właściwości:**

| Właściwość | Zastosowanie |
|------------|--------------|
| `accessibilityLabel` | Opis elementu dla screen readera |
| `accessibilityHint` | Dodatkowa wskazówka o działaniu |
| `accessibilityRole` | Semantyczna rola (button, header, link) |
| `accessibilityState` | Stan elementu (disabled, selected, checked) |

**Typowe błędy:**
- Brak etykiet na ikonach (screen reader mówi „przycisk" bez opisu)
- Słabe zarządzanie focusem (nawigacja klawiaturą nie działa)
- Gesty bez alternatyw (swipe bez przycisku)

**Testowanie:**
- VoiceOver na iOS (Settings → Accessibility → VoiceOver)
- TalkBack na Android (Settings → Accessibility → TalkBack)
- Nawigacja tylko klawiaturą (podłączona klawiatura Bluetooth)

---

## Workflow audytu dostępności

**Krok 1: Automatyczne narzędzia**
- axe DevTools — wykrywa typowe problemy
- Lighthouse accessibility score — ogólna ocena
- Color contrast checkers — weryfikacja kontrastów

**Krok 2: Testowanie manualne**
- Nawigacja tylko klawiaturą
- Testowanie z włączonym screen readerem (VoiceOver, TalkBack)
- Weryfikacja przy różnych rozmiarach czcionki systemowej

**Krok 3: Testy z użytkownikami**
- 2-3 osoby z różnymi potrzebami dostępności
- Obserwacja rzeczywistego użytkowania
- Zbieranie feedbacku jakościowego

**Krok 4: Dokumentacja i śledzenie**
- Katalog znalezionych problemów
- Priorytetyzacja (krytyczne → niskie)
- Regularna walidacja po zmianach

---

## Ustawienia dostępności w aplikacji

Oprócz zgodności z WCAG, warto dodać dedykowane ustawienia dostępności.

**Rekomendowane opcje:**

| Ustawienie | Opis |
|------------|------|
| Rozmiar czcionki | Możliwość zwiększenia tekstu |
| Wysoki kontrast | Zwiększony kontrast kolorów |
| Redukcja ruchu | Wyłączenie animacji |
| Tryb ciemny | Często preferowany przez osoby z wrażliwością na światło |

---

## Dostępność jako cecha produktu

Dostępność nie powinna być prezentowana jako „wsparcie dla niepełnosprawnych", ale jako uniwersalny design.

**Komunikacja:**
- „Poziomki zaprojektowane dla wszystkich"
- Nie „accessibility" jako checkbox do odhaczenia
- Dostępność jako naturalny element jakości produktu

**W materiałach promocyjnych:**
- Wspomnieć o zgodności z WCAG 2.1 AA
- Wyróżnić ustawienia dostępności jako feature
- Podkreślić, że to standard, nie wyjątek

---

## Źródła

- W3C, *Web Content Accessibility Guidelines (WCAG) 2.1*
- WHO, *World Report on Disability*
- React Native, *Accessibility Documentation*
- Apple, *Human Interface Guidelines — Accessibility*
- Google, *Material Design — Accessibility*
- WebAIM, *Mobile Accessibility Testing*
