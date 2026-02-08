# Projektowanie dla pokolenia Z

## Zadania

1. **Luty 2026**: Zaimplementować tryb ciemny jako domyślny (0.2.0) z możliwością przełączenia na jasny
2. **Marzec 2026**: Przeprowadzić audyt dostępności WCAG 2.1 AA (wymagane dla grantu NLnet)
3. **Marzec 2026**: Zmienić ikony na bibliotekę Phosphor (zgodnie z ROADMAP.md)
4. **Kwiecień 2026**: Przeprowadzić 5-10 testów użyteczności z prawdziwymi studentami
5. **Kwiecień 2026**: Wdrożyć personalizację kolorów profilu (gradienty, Material You)

---

## Trendy w projektowaniu interfejsów 2025-2026

Projektowanie dla pokolenia Z (urodzeni 1997-2012) wymaga zrozumienia ich estetycznych preferencji i nawyków korzystania z technologii. Badania UX z lat 2024-2025 wskazują na kilka dominujących trendów.

**Neubrutalizm:**
- Ostre krawędzie, płaskie kolory, bold typografia
- Surowe, „niewypolerowane" layouty kontrastujące z korporacyjnym designem
- Przemawia do Gen Z ceniących autentyczność i bezpretensjonalność
- Przykłady: Figma, Notion, nowa identyfikacja Spotify

**Glassmorphism:**
- Efekt matowego szkła z rozmytym tłem
- Lekka, nowoczesna estetyka budująca wrażenie przestrzeni
- Wraca do mody po kilku latach przerwy

**Personalizacja:**
- 80% użytkowników preferuje spersonalizowane doświadczenia (McKinsey, 2024)
- Material You (Android 12+) — dynamiczne kolory dopasowane do tapety
- Możliwość wyboru akcentów kolorystycznych, gradientów w profilu

---

## Nawigacja i ergonomia

**Nawigacja dolna jako standard:**
Badania UX Movement (2023) wykazały, że dolna nawigacja jest o 21% szybsza w użyciu niż górna. Dla aplikacji mobilnych to oczywisty wybór.

**Projektowanie dla jednej ręki:**
- 75% użytkowników obsługuje telefon jedną ręką (Steven Hoober, 2023)
- Kluczowe akcje powinny być w zasięgu kciuka (dolna część ekranu)
- Przyciski akcji minimum 44x44 pikseli (wytyczne Apple HIG)

**Strefa kciuka (thumb zone):**
- Łatwo dostępna: dolne 1/3 ekranu
- Średnio dostępna: środek ekranu
- Trudno dostępna: górne rogi

---

## Tryb ciemny — standard, nie opcja

Tryb ciemny przestał być funkcją premium — jest oczekiwanym standardem, szczególnie wśród młodych użytkowników.

**Dlaczego tryb ciemny powinien być domyślny:**
- Zmniejsza zmęczenie oczu przy długim korzystaniu, szczególnie wieczorem
- Oszczędza baterię na ekranach OLED (nawet o 30-40%)
- Tworzy wrażenie intymności — ważne dla aplikacji społecznościowej
- Twitter/X i Reddit odnotowały znaczny wzrost sesji wieczornych po wprowadzeniu dark mode

**Implementacja:**
- Tryb ciemny jako domyślny przy pierwszym uruchomieniu
- Łatwe przełączanie w ustawieniach
- Respektowanie ustawień systemowych (prefers-color-scheme)

---

## Dostępność — wymóg, nie bonus

Dostępność (accessibility) to nie opcjonalny dodatek, ale fundamentalny wymóg projektowy. Grant NLnet wymaga zgodności z WCAG 2.1 AA.

**Statystyki:**
- 15% światowej populacji żyje z jakąś formą niepełnosprawności (WHO)
- 73% użytkowników z niepełnosprawnościami opuszcza strony i aplikacje, które są trudne w obsłudze
- Inclusive design zwiększa zaangażowanie wszystkich użytkowników o 35%

**Podstawowe wymagania WCAG 2.1 AA:**
- Kontrast tekstu minimum 4.5:1 (3:1 dla dużego tekstu)
- Wszystkie funkcje dostępne z klawiatury
- Tekst alternatywny dla obrazów
- Czytelne etykiety formularzy
- Brak treści migających szybciej niż 3 razy na sekundę

Szczegółowy audyt dostępności opisany w dokumencie [22-accessibility-wcag.md](22-accessibility-wcag.md).

---

## Etyczny design — sukces to wyjście z aplikacji

Poziomki różnią się od typowych aplikacji społecznościowych fundamentalną filozofią: sukces to spotkanie w realu, nie maksymalizacja czasu ekranowego.

**Implikacje dla designu:**
- Nie stosować dark patterns (nieskończone scrollowanie, FOMO notifications)
- Ułatwiać umawianie spotkań, nie przeszkadzać powiadomieniami
- Pokazywać jasno „zostało umówione spotkanie" zamiast zachęcać do dalszego przeglądania
- Transparentność w wykorzystaniu danych użytkownika

**Zrównoważony design:**
- Efektywny kod = szybsze ładowanie = mniejsze zużycie baterii
- Kompresja obrazów i lazy loading
- Minimalizm funkcjonalny — tylko to, co naprawdę potrzebne

---

## Źródła

- McKinsey, *The Value of Personalization* (2024)
- UX Movement, *Bottom Navigation vs Top Navigation* (2023)
- Steven Hoober, *How Do Users Really Hold Mobile Devices?* (2023)
- Apple Human Interface Guidelines, *Touch Targets*
- W3C, *Web Content Accessibility Guidelines (WCAG) 2.1*
- WHO, *World Report on Disability*
