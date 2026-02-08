# Zrównoważoność zespołu — 3 studentów

## Zadania

1. **Luty 2026**: Ustalić realistyczne oczekiwania i jasny podział obowiązków w zespole
2. **Marzec 2026**: Zdefiniować „minimum viable effort" na różne fazy projektu
3. **Kwiecień 2026**: Wprowadzić regularne check-iny o samopoczuciu i work-life balance
4. **2026**: Dokumentować wszystko (mitygacja bus factor)
5. **2026**: Przygotować plan na wakacje, sesje i inne wydarzenia życiowe

---

## Wypalenie założycieli — statystyki (2024-2025)

Problem wypalenia jest powszechny nawet wśród profesjonalnych założycieli startupów. Dla studentów łączących projekt ze studiami ryzyko jest jeszcze większe.

**Dane branżowe:**

| Statystyka | Wartość |
|------------|---------|
| Wypalenie w ostatnim roku | 54% założycieli |
| Wypalenie kiedykolwiek | 72% założycieli |
| Praca 50+ godzin tygodniowo | 67% założycieli |
| Upadek startupu z powodu wypalenia | 5% |
| Wysoki poziom stresu | 83% założycieli |
| Bezsenność | 54% założycieli |

**Kontekst Poziomek:**
Zespół 3 studentów = side-project, nie pełnoetatowy startup. Ale te same mechanizmy prowadzą do wypalenia w mniejszej skali. Studia + projekt + życie prywatne = potencjalnie za dużo.

---

## Mentalność side-project (nie 9-9-6)

Poziomki to projekt studencki, nie startup z inwestorem oczekującym wzrostu za wszelką cenę.

**Realistyczne godziny:**
- 5-15 godzin tygodniowo na osobę w fazie rozwoju
- Mniej w okresie sesji
- Elastyczność dostosowana do obciążenia na studiach

**Priorytetyzacja:**
Studia > projekt. Studia mają deadline'y i konsekwencje (skreślenie z listy studentów). Projekt może poczekać.

**Intensywne okresy vs zrównoważone tempo:**
Intensywne sprinty przed release'em są OK, ale muszą być followed by recovery. Ciągły sprint = wypalenie.

---

## Podział obowiązków

Jasne ownership zapobiega duplikowaniu pracy i „rozmyciu" odpowiedzialności.

**Zasady:**
- Każdy obszar ma primary ownera (backend, mobile, design, operacje)
- Backup dla krytycznych obszarów (co jeśli ktoś zachoruje?)
- Decyzje podejmuje owner obszaru, nie komitet

**Szybkie decyzje:**
Szybkie decyzje > perfekcyjne decyzje. Paraliż analityczny to realne ryzyko w małych zespołach. Ustalić kto decyduje o czym, żeby uniknąć nieskończonych dyskusji.

**Komunikacja:**
- Regularne synci (raz w tygodniu?)
- Async-first (nie wszystko wymaga spotkania)
- Dokumentacja decyzji (żeby nie zapomnieć dlaczego coś zrobiliśmy)

---

## Mitygacja bus factor

Bus factor = „co się stanie, jeśli jedna osoba wypadnie?". Dla zespołu 3 osób to krytyczne ryzyko.

**Dokumentacja:**
- Każda decyzja architektoniczna zapisana
- Setup środowiska udokumentowany
- Procesy (jak robimy release, jak moderujemy) opisane
- Nowa osoba może się wdrożyć bez „ustnej historii"

**Jakość kodu:**
- Czytelny kod (nie tylko dla autora)
- Testy (ktoś inny może zweryfikować czy działa)
- CI/CD (automat sprawdza podstawy)

**Zaleta open-source:**
Projekt może żyć bez założycieli. Społeczność może przejąć rozwój. To nie porażka — to plan sukcesji.

---

## Planowanie wydarzeń życiowych

Życie się zdarza. Lepiej mieć plan niż reagować ad hoc.

**Sesje egzaminacyjne (styczeń-luty, czerwiec):**
- **Sesja = pauza w rozwoju** — to normalne i oczekiwane
- Minimum viable maintenance
- Tylko bugfixy, żadnych nowych funkcji
- Jasna komunikacja do społeczności: „wracamy po sesji"
- Nie planować żadnych deadline'ów na okres sesji

**Wakacje:**
- Rotacja — ktoś zawsze „on call", ale minimalnie
- Automatyczne odpowiedzi na issues/PR
- Zaplanowane przerwy, nie spontaniczne zniknięcia

**Nieoczekiwane sytuacje:**
- Co jeśli ktoś musi odejść z projektu?
- Plan B: kto przejmie obowiązki?
- Dokumentacja onboardingowa dla potencjalnego nowego członka

---

## Sygnały ostrzegawcze wypalenia

Wczesne rozpoznanie pozwala zareagować, zanim będzie za późno.

**Na co zwracać uwagę:**
- Odpowiadanie na wiadomości w nocy regularnie
- Opuszczanie zajęć na studiach dla projektu
- Poczucie urazy wobec projektu lub zespołu
- Brak radości z postępów
- Ciągłe zmęczenie

**Co robić gdy zauważysz sygnały:**
1. Zatrzymać się (pauza, nie push through)
2. Przeanalizować obciążenie (co można zredukować?)
3. Dostosować zakres (wolniejszy progress > wypalenie)
4. Porozmawiać z zespołem (nie cierpieć w milczeniu)

**Zasada:**
Wolniejszy postęp jest lepszy niż wypalenie i porzucenie projektu.

---

## Strategia wyjścia (nie porażka)

Co jeśli zespół nie może kontynuować? To nie musi być katastrofa.

**Opcje honorowego zakończenia:**

| Opcja | Opis |
|-------|------|
| Fork przez społeczność | Open-source = inni mogą kontynuować |
| Przekazanie innemu zespołowi | Dokumentacja umożliwia handover |
| Sunsetting z terminem | Ogłoszenie zakończenia z wyprzedzeniem |
| Archiwizacja | Nie usuwać, zachować jako zasób |

**Framing:**
Zakończenie projektu z planem to nie porażka. Wiele projektów open-source jest „finished" lub przekazanych. Lepsze honorowe zakończenie niż cicha śmierć.

**Dokumentacja na wypadek wyjścia:**
- Stan projektu (co działa, co nie)
- Znane problemy i tech debt
- Instrukcja przejęcia dla potencjalnego następcy

---

## Źródła

- Startup Snapshot, *Founder Mental Health Report 2024*
- First Round Review, *Founder Burnout Articles*
- Open Source Collective, *Maintainer Burnout Prevention*
- Cal Newport, *Deep Work* (o zrównoważonej produktywności)
