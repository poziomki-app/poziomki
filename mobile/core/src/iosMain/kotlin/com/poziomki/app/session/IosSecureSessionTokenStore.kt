package com.poziomki.app.session

import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.alloc
import kotlinx.cinterop.get
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.reinterpret
import kotlinx.cinterop.toCValues
import kotlinx.cinterop.value
import platform.CoreFoundation.CFDataCreate
import platform.CoreFoundation.CFDataGetBytePtr
import platform.CoreFoundation.CFDataGetLength
import platform.CoreFoundation.CFDictionaryCreateMutable
import platform.CoreFoundation.CFDictionaryRef
import platform.CoreFoundation.CFDictionarySetValue
import platform.CoreFoundation.CFRelease
import platform.CoreFoundation.CFStringCreateWithCString
import platform.CoreFoundation.CFTypeRefVar
import platform.CoreFoundation.kCFAllocatorDefault
import platform.CoreFoundation.kCFBooleanTrue
import platform.CoreFoundation.kCFStringEncodingUTF8
import platform.Security.SecItemAdd
import platform.Security.SecItemCopyMatching
import platform.Security.SecItemDelete
import platform.Security.errSecSuccess
import platform.Security.kSecAttrAccessible
import platform.Security.kSecAttrAccessibleWhenUnlockedThisDeviceOnly
import platform.Security.kSecAttrAccount
import platform.Security.kSecAttrService
import platform.Security.kSecClass
import platform.Security.kSecClassGenericPassword
import platform.Security.kSecMatchLimit
import platform.Security.kSecMatchLimitOne
import platform.Security.kSecReturnData
import platform.Security.kSecValueData

@OptIn(ExperimentalForeignApi::class)
class IosSecureSessionTokenStore : SessionTokenStore {
    override suspend fun getToken(): String? =
        withBaseQuery(includeAccessible = false) { query ->
            memScoped {
                CFDictionarySetValue(query, kSecReturnData, kCFBooleanTrue)
                CFDictionarySetValue(query, kSecMatchLimit, kSecMatchLimitOne)

                val result = alloc<CFTypeRefVar>()
                val status = SecItemCopyMatching(query, result.ptr)
                if (status != errSecSuccess) {
                    return@memScoped null
                }

                val dataRef = result.value?.reinterpret<cnames.structs.__CFData>() ?: return@memScoped null
                try {
                    dataRef.toUtf8String()
                } finally {
                    CFRelease(dataRef)
                }
            }
        }

    override suspend fun saveToken(token: String) {
        clearToken()
        val data = token.toCFData() ?: return
        try {
            withBaseQuery(includeAccessible = true) { query ->
                CFDictionarySetValue(query, kSecValueData, data)
                SecItemAdd(query, null)
            }
        } finally {
            CFRelease(data)
        }
    }

    override suspend fun clearToken() {
        withBaseQuery(includeAccessible = false) { query ->
            SecItemDelete(query)
        }
    }

    private inline fun <T> withBaseQuery(
        includeAccessible: Boolean,
        block: (CFDictionaryRef) -> T,
    ): T? {
        val query = CFDictionaryCreateMutable(kCFAllocatorDefault, 0, null, null) ?: return null
        val service = CFStringCreateWithCString(kCFAllocatorDefault, SERVICE, kCFStringEncodingUTF8)
        val account = CFStringCreateWithCString(kCFAllocatorDefault, ACCOUNT, kCFStringEncodingUTF8)
        if (service == null || account == null) {
            service?.let(::CFRelease)
            account?.let(::CFRelease)
            CFRelease(query)
            return null
        }

        return try {
            CFDictionarySetValue(query, kSecClass, kSecClassGenericPassword)
            CFDictionarySetValue(query, kSecAttrService, service)
            CFDictionarySetValue(query, kSecAttrAccount, account)
            if (includeAccessible) {
                CFDictionarySetValue(
                    query,
                    kSecAttrAccessible,
                    kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
                )
            }
            block(query)
        } finally {
            CFRelease(account)
            CFRelease(service)
            CFRelease(query)
        }
    }

    private fun String.toCFData(): platform.CoreFoundation.CFDataRef? {
        val bytes = encodeToByteArray().toUByteArray()
        return CFDataCreate(kCFAllocatorDefault, bytes.toCValues(), bytes.size.toLong())
    }

    private fun platform.CoreFoundation.CFDataRef.toUtf8String(): String? {
        val length = CFDataGetLength(this).toInt()
        val bytes = CFDataGetBytePtr(this) ?: return null
        return ByteArray(length) { index -> bytes[index].toByte() }.decodeToString()
    }

    private companion object {
        const val SERVICE = "com.poziomki.app.session"
        const val ACCOUNT = "session_token"
    }
}
