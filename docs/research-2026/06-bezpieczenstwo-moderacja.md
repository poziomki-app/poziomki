# Bezpieczeństwo i moderacja treści

## Zadania

1. **Luty 2026**: Wdrożyć niezawodny SMTP dla kodów OTP/2FA (ROADMAP 0.2.0)
2. **Marzec 2026**: Napisać Community Guidelines i procedury reagowania przed udostępnieniem kodu
3. **Marzec 2026**: Zaimplementować szyfrowanie czatów (OMEMO lub Double Ratchet)
4. **Kwiecień 2026**: Wdrożyć system zgłaszania naruszeń z przejrzystym workflow (ROADMAP 0.3.0)
5. **Maj 2026**: Przygotować dokumentację RODO i procedurę powiadamiania o naruszeniach (72h) przed publikacją w Google Play

---

## RODO/GDPR — obowiązek od pierwszego dnia

Rozporządzenie o Ochronie Danych Osobowych (RODO/GDPR) obowiązuje każdą organizację przetwarzającą dane osobowe w UE, niezależnie od wielkości.

**Podstawowe zasady:**
- Każde przetwarzanie danych wymaga podstawy prawnej (zgoda, umowa, prawnie uzasadniony interes)
- Dane osobowe tylko w zakresie niezbędnym do celu (minimalizacja)
- Przechowywanie tylko przez czas niezbędny
- Privacy by design & by default — ochrona wbudowana w system

**Kary za naruszenia:**
- Maksymalna kara: 20 milionów euro lub 4% rocznego obrotu
- Morele.net: 660 000 euro za wyciek danych klientów (2019)
- Toyota Bank Polska: 78 000 zł za opóźnione zgłoszenie naruszenia

**Procedura zgłaszania naruszeń:**
- 72 godziny na zgłoszenie naruszenia do UODO od momentu jego wykrycia
- Jeśli naruszenie może powodować wysokie ryzyko dla osób — powiadomienie również poszkodowanych
- Dokumentacja wszystkich naruszeń, nawet niezgłoszonych

**Dla Poziomek:**
- Przygotować rejestr czynności przetwarzania
- Napisać politykę prywatności w przystępnym języku
- Wdrożyć procedurę breach notification
- Umożliwić eksport i usunięcie danych użytkownika (prawo do przenoszenia i bycia zapomnianym)

---

## Akt o Usługach Cyfrowych (DSA)

Digital Services Act (DSA) to unijne rozporządzenie regulujące platformy internetowe, obowiązujące od lutego 2024 roku.

**Dobre wiadomości dla małych platform:**
Mikro i małe przedsiębiorstwa (poniżej 50 pracowników i 10 mln euro obrotu) są zwolnione z najcięższych obowiązków DSA:
- Nie muszą przeprowadzać zewnętrznych audytów
- Nie muszą publikować formalnych raportów transparentności
- Nie podlegają bezpośredniemu nadzorowi Komisji Europejskiej

**Nadal wymagane dla wszystkich platform:**
- Jasne warunki korzystania z usługi w przystępnym języku
- Procedury moderacji treści i odwołania od decyzji
- Mechanizm zgłaszania nielegalnych treści
- Powiadamianie użytkowników o decyzjach moderacyjnych z uzasadnieniem

Każdy kraj UE ma wyznaczonego Digital Services Coordinator jako regulatora — w Polsce będzie to najprawdopodobniej UKE lub UOKiK.

---

## Lekcje z innych kampusowych aplikacji

System UNC (University of North Carolina) w 2024 roku zakazał aplikacji Yik Yak, Sidechat, Fizz i Whisper z powodu cyberprzemocy, handlu narkotykami i molestowania seksualnego. To jedna z pierwszych takich decyzji na uczelniach w USA.

**Wspólny mianownik problematycznych aplikacji:** anonimowość postów.

**Wnioski dla Poziomek:**
- Unikać pełnej anonimowości — weryfikacja email uczelnianym
- Wszystkie posty/wiadomości muszą być przypisane do konta
- Szybka reakcja na zgłoszenia = kluczowa dla reputacji
- Współpraca z administracją uczelni (nie przeciwko niej)

---

## Moderacja treści — podejście hybrydowe

Skuteczna moderacja treści wymaga połączenia automatyzacji z ludzką oceną.

**Zalety moderacji automatycznej (AI):**
- Przetwarza treści tysiące razy szybciej niż człowiek
- Skuteczna przy oczywistych naruszeniach: spam, wulgaryzmy, nagość
- Badania wskazują na 30% wzrost satysfakcji użytkowników przy szybkiej moderacji

**Ograniczenia AI:**
- Słabo radzi sobie z sarkazmem, satyrą, kontekstem kulturowym
- Może błędnie oznaczać dopuszczalne treści (false positives)
- Wymaga ciągłego doskonalenia i nadzoru

**Rekomendowany model dla Poziomek:**
1. **Automatyczny filtr** — wykrywanie spamu, profanity filter, skanowanie obrazów pod kątem NSFW
2. **Zgłoszenia użytkowników** — przycisk „zgłoś" przy każdej treści
3. **Przegląd przez zespół** — decyzje w trudnych przypadkach, odwołania
4. **Przejrzyste zasady** — Community Guidelines dostępne przed rejestracją

---

## Szyfrowanie czatów

Szyfrowanie end-to-end (E2EE) to standard prywatności w komunikatorach. Użytkownicy oczekują, że ich prywatne rozmowy pozostaną prywatne.

**Signal Protocol — złoty standard:**
- Używany przez WhatsApp, Google Messages, Facebook Messenger
- Łączy szyfrowanie asymetryczne z forward secrecy
- Referencyjna implementacja dostępna w Rust (licencja AGPLv3)
- Double Ratchet zapewnia, że kompromitacja jednego klucza nie ujawnia poprzednich wiadomości

**Alternatywy open-source:**
- **OMEMO** — rozszerzenie protokołu XMPP, dojrzała implementacja
- **Matrix Olm/Megolm** — protokół szyfrowania dla Matrix, dobrze udokumentowany
- **Wire** — własna implementacja oparta na Signal Protocol

**Rekomendacja dla Poziomek:**
Rozważyć Matrix Olm lub OMEMO — obie opcje mają istniejące biblioteki open-source i są łatwiejsze do wdrożenia niż implementacja Signal Protocol od zera. Pełne E2EE zaplanowane na wersję 0.4.0 zgodnie z ROADMAP.

---

## Uwierzytelnianie dwuskładnikowe (2FA)

Dwuskładnikowe uwierzytelnianie (2FA) znacząco podnosi bezpieczeństwo kont użytkowników.

**Dlaczego 2FA jest konieczne:**
- Większość włamań na konta wynika z ponownego użycia haseł (credential stuffing)
- 2FA blokuje 99,9% automatycznych ataków (Microsoft, 2023)
- Użytkownicy oczekują 2FA jako standardu bezpieczeństwa

**Opcje implementacji:**
- **OTP przez email** — najprostsze, ale wymaga niezawodnego SMTP
- **TOTP (aplikacja)** — Google Authenticator, Authy — bezpieczniejsze, nie wymaga internetu
- **WebAuthn/Passkeys** — najbezpieczniejsze, coraz szerzej wspierane

**Dla Poziomek:**
Zacząć od OTP przez email (0.2.0), rozważyć TOTP jako opcję w późniejszych wersjach.

---

## Przejrzyste zasady społeczności

Community Guidelines powinny być gotowe przed publicznym udostępnieniem aplikacji.

**Elementy Community Guidelines:**
- Co jest dozwolone i czego zabraniamy (jasne przykłady)
- Konsekwencje naruszeń (ostrzeżenie, zawieszenie, ban)
- Jak zgłaszać naruszenia
- Proces odwoławczy

**System zgłoszeń:**
- Przycisk „zgłoś" przy każdym profilu, wiadomości, wydarzeniu
- Kategorie zgłoszeń (spam, nękanie, nieodpowiednie treści, inne)
- Potwierdzenie przyjęcia zgłoszenia dla zgłaszającego
- Informacja o wyniku rozpatrzenia (bez ujawniania szczegółów)

---

## Źródła

- UODO, *Poradnik RODO dla przedsiębiorców*
- Komisja Europejska, *Digital Services Act — pytania i odpowiedzi*
- Signal Foundation, *Signal Protocol Documentation*
- Matrix.org, *Olm/Megolm Specification*
- Microsoft Security, *Your Pa$$word doesn't matter* (2023)
- [Inside Higher Ed, *UNC system banning anonymous social apps* (2024)](https://www.insidehighered.com/news/tech-innovation/teaching-learning/2024/03/13/unc-system-banning-anonymous-social-apps-over)
