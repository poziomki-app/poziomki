package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

/** Full-screen scrollable dialog for a legal document (terms / privacy). */
@Composable
internal fun LegalDocumentDialog(
    title: String,
    body: String,
    onDismiss: () -> Unit,
) {
    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Surface(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            shape = RoundedCornerShape(20.dp),
            color = MaterialTheme.colorScheme.background,
        ) {
            Column(
                modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
            ) {
                Text(
                    text = title,
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 22.sp,
                    color = TextPrimary,
                )
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = body,
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                    lineHeight = 22.sp,
                )
                Spacer(modifier = Modifier.height(24.dp))
                AppButton(
                    text = "zamknij",
                    onClick = onDismiss,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

internal val regulaminText =
    """
    Niniejszy Regulamin określa zasady korzystania z aplikacji Poziomki oraz prawa i obowiązki użytkowników.

    1. Akceptacja regulaminu
    Korzystając z aplikacji Poziomki, akceptujesz niniejszy Regulamin oraz Politykę prywatności. Jeśli nie zgadzasz się z którymkolwiek z postanowień, nie korzystaj z aplikacji.

    2. Wymagania
    Z aplikacji mogą korzystać wyłącznie osoby, które ukończyły 16 lat. Zakładając konto, oświadczasz, że podane dane są prawdziwe.

    3. Treści użytkowników
    Aplikacja umożliwia publikowanie treści (zdjęcia profilowe, opisy, wydarzenia, wiadomości). Ponosisz pełną odpowiedzialność za treści, które publikujesz, oraz za swoje zachowanie wobec innych użytkowników.

    4. Zakaz treści niedozwolonych
    Obowiązuje zasada zera tolerancji dla treści i zachowań niedozwolonych. Zabronione jest publikowanie oraz przesyłanie treści, które są: obraźliwe, nękające, nawołujące do nienawiści, dyskryminujące, brutalne, o charakterze seksualnym, nielegalne, wprowadzające w błąd, spamerskie lub naruszające prawa innych osób. Zabronione jest również nękanie, zastraszanie i podszywanie się pod inne osoby.

    5. Zgłaszanie i blokowanie
    Każdy użytkownik może zgłosić niewłaściwe treści oraz zablokować innego użytkownika bezpośrednio w aplikacji. Zablokowanie użytkownika natychmiast usuwa jego treści z Twojego widoku i powiadamia zespół Poziomki.

    6. Moderacja
    Zgłoszenia są rozpatrywane bez zbędnej zwłoki, zwykle w ciągu 24 godzin. Treści naruszające Regulamin są usuwane, a konta użytkowników dopuszczających się naruszeń mogą zostać zawieszone lub trwale usunięte.

    7. Spotkania i wydarzenia
    Aplikacja umożliwia organizację i udział w spotkaniach. Uczestnictwo w wydarzeniach odbywa się na własną odpowiedzialność. Zachowaj zdrowy rozsądek i ostrożność podczas spotkań z osobami poznanymi w aplikacji.

    8. Zawieszenie i usunięcie konta
    Zespół Poziomki może zawiesić lub usunąć konto użytkownika naruszającego Regulamin. Użytkownik może w każdej chwili usunąć własne konto w ustawieniach aplikacji.

    9. Zmiany regulaminu
    O istotnych zmianach Regulaminu użytkownicy zostaną poinformowani poprzez powiadomienie w aplikacji.

    10. Kontakt
    Pytania dotyczące Regulaminu prosimy kierować na adres: kontakt@poziomki.app

    Data ostatniej aktualizacji: czerwiec 2026
    """.trimIndent()

internal val privacyPolicyText =
    """
    Niniejsza Polityka Prywatności określa zasady przetwarzania danych osobowych użytkowników aplikacji Poziomki.

    1. Administrator danych
    Administratorem danych osobowych jest zespół Poziomki. Kontakt: kontakt@poziomki.app

    2. Zakres zbieranych danych
    Zbieramy następujące dane: adres e-mail, imię, zdjęcia profilowe, zainteresowania oraz dane dotyczące uczestnictwa w wydarzeniach.

    3. Cel przetwarzania
    Dane przetwarzamy w celu: świadczenia usług aplikacji, dopasowywania rekomendacji wydarzeń i profili, komunikacji między użytkownikami oraz zapewnienia bezpieczeństwa.

    4. Udostępnianie danych
    Dane osobowe nie są sprzedawane ani udostępniane podmiotom trzecim w celach marketingowych. Dane mogą być udostępniane wyłącznie na żądanie organów uprawnionych na podstawie przepisów prawa.

    5. Przechowywanie danych
    Dane przechowywane są na serwerach zlokalizowanych w Unii Europejskiej. Dane są przechowywane przez okres korzystania z aplikacji oraz do 30 dni po usunięciu konta.

    6. Prawa użytkownika
    Każdy użytkownik ma prawo do: dostępu do swoich danych, ich sprostowania, usunięcia, ograniczenia przetwarzania, przenoszenia danych oraz wniesienia sprzeciwu. Eksport i usunięcie danych dostępne są w ustawieniach aplikacji.

    7. Pliki cookies i analityka
    Aplikacja nie wykorzystuje plików cookies. Zbieramy anonimowe dane analityczne w celu poprawy jakości usług.

    8. Zmiany polityki
    O istotnych zmianach w polityce prywatności użytkownicy zostaną poinformowani poprzez powiadomienie w aplikacji.

    9. Kontakt
    Pytania dotyczące prywatności prosimy kierować na adres: kontakt@poziomki.app

    Data ostatniej aktualizacji: marzec 2026
    """.trimIndent()
