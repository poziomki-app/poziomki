package com.poziomki.app.di

import android.content.Context
import androidx.sqlite.db.SupportSQLiteDatabase
import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.android.AndroidSqliteDriver
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.push.NotificationHelper
import com.poziomki.app.chat.push.PushManager
import com.poziomki.app.chat.ws.WsChatClient
import com.poziomki.app.connectivity.AndroidConnectivityMonitor
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.session.AndroidSecureSessionTokenStore
import com.poziomki.app.session.SessionTokenStore
import com.poziomki.app.session.createDataStore
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.okhttp.OkHttp
import okhttp3.CertificatePinner
import org.koin.core.module.Module
import org.koin.dsl.module

/**
 * Certificate pinning for the Poziomki API. Pins the Let's Encrypt
 * intermediates (E7 is what the prod leaf currently chains through)
 * plus both ISRG roots as fallbacks so a routine LE rotation onto a
 * different intermediate doesn't brick the app.
 *
 * Rotation runbook:
 * 1. Inspect the live chain: `openssl s_client -connect api.poziomki.app:443
 *    -showcerts </dev/null`
 * 2. For each cert in the chain, compute its SPKI pin:
 *    `openssl x509 -in cert.pem -pubkey -noout | openssl pkey -pubin -outform DER |
 *     openssl dgst -sha256 -binary | openssl base64`
 * 3. Add the new intermediate pin to API_CERT_PINS (keep the old one
 *    during the rollover window so already-installed builds still
 *    connect).
 * 4. Ship a new APK before the old pin's intermediate rotates out.
 */
private val API_CERT_PINS =
    listOf(
        // Let's Encrypt E7 (current prod intermediate, ECDSA P-384).
        "sha256/y7xVm0TVJNahMr2sZydE2jQH8SquXV9yLF9seROHHHU=",
        // ISRG Root X1 (RSA 4096) — LE's primary trust anchor.
        "sha256/C5+lpZ7tcVwmwQIMcRtPbsQtWLABXhQzejna0wHFr8M=",
        // ISRG Root X2 (ECDSA P-384) — LE's secondary trust anchor.
        "sha256/diGVwiVYbubAI3RW4hB9xU8e/CH2GnkuvVFZE8zmgzI=",
    )

private const val API_HOST = "api.poziomki.app"

private fun buildApiCertificatePinner(): CertificatePinner {
    val builder = CertificatePinner.Builder()
    API_CERT_PINS.forEach { pin -> builder.add(API_HOST, pin) }
    return builder.build()
}

actual fun platformModule(): Module =
    module {
        single<HttpClientEngine> {
            val apiUrl = getProperty("API_BASE_URL", "http://localhost:5150")
            // Only pin when the app is actually talking to the prod
            // API — dev builds hit http://localhost and would hit a
            // different cert chain under any HTTPS dev setup. The
            // pin is scoped to api.poziomki.app only; any other host
            // the client hits goes through the normal trust store.
            val pinningEnabled = apiUrl.startsWith("https://$API_HOST")
            OkHttp.create {
                addInterceptor { chain ->
                    val request =
                        chain
                            .request()
                            .newBuilder()
                            .header("Origin", apiUrl)
                            .build()
                    chain.proceed(request)
                }
                if (pinningEnabled) {
                    config {
                        certificatePinner(buildApiCertificatePinner())
                    }
                }
            }
        }
        single { createDataStore(get<Context>()) }
        single<SessionTokenStore> { AndroidSecureSessionTokenStore(get<Context>()) }
        single<ChatClient> { WsChatClient(get(), get(), get(), get()) }
        single<SqlDriver> {
            val context = get<Context>()
            val dbName = "poziomki.db"
            val schema = PoziomkiDatabase.Schema
            val callback =
                object : AndroidSqliteDriver.Callback(schema) {
                    override fun onUpgrade(
                        db: SupportSQLiteDatabase,
                        oldVersion: Int,
                        newVersion: Int,
                    ) {
                        try {
                            super.onUpgrade(db, oldVersion, newVersion)
                        } catch (_: Exception) {
                            val cursor =
                                db.query(
                                    "SELECT name FROM sqlite_master WHERE type='table'" +
                                        " AND name NOT LIKE 'sqlite_%'" +
                                        " AND name NOT LIKE 'android_%'",
                                )
                            val tables = mutableListOf<String>()
                            while (cursor.moveToNext()) {
                                tables.add(cursor.getString(0))
                            }
                            cursor.close()
                            tables.forEach { db.execSQL("DROP TABLE IF EXISTS \"$it\"") }
                            onCreate(db)
                        }
                    }
                }
            AndroidSqliteDriver(
                schema = schema,
                context = context,
                name = dbName,
                callback = callback,
            )
        }
        single<ConnectivityMonitor> { AndroidConnectivityMonitor(get<Context>()) }
        single { LocationProvider(get<Context>()) }
        single { NotificationHelper(get<Context>()) }
        single { PushManager(get(), get()) }
    }
