package com.poziomki.app.session

import kotlinx.cinterop.CFTypeRefVar
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import platform.CoreFoundation.CFDictionaryRef
import platform.Foundation.NSData
import platform.Foundation.NSString
import platform.Foundation.NSUTF8StringEncoding
import platform.Foundation.create
import platform.Foundation.dataUsingEncoding
import platform.Foundation.stringWithUTF8String
import platform.Security.SecItemAdd
import platform.Security.SecItemCopyMatching
import platform.Security.SecItemDelete
import platform.Security.errSecSuccess
import platform.Security.kSecAttrAccount
import platform.Security.kSecAttrService
import platform.Security.kSecClass
import platform.Security.kSecClassGenericPassword
import platform.Security.kSecMatchLimit
import platform.Security.kSecMatchLimitOne
import platform.Security.kSecReturnData
import platform.Security.kSecValueData

class IosSecureSessionTokenStore : SessionTokenStore {
    override suspend fun getToken(): String? =
        memScoped {
            val query =
                baseQuery().apply { put(kSecReturnData, true) }.apply {
                    put(kSecMatchLimit, kSecMatchLimitOne)
                }
            val result = alloc<CFTypeRefVar>()
            val status = SecItemCopyMatching(query as CFDictionaryRef, result.ptr)
            if (status != errSecSuccess) {
                return@memScoped null
            }
            val data = result.value as? NSData ?: return@memScoped null
            NSString.create(data, NSUTF8StringEncoding)?.toString()
        }

    override suspend fun saveToken(token: String) {
        clearToken()
        val data = token.toNSData() ?: return
        val query = baseQuery().apply { put(kSecValueData, data) }
        SecItemAdd(query as CFDictionaryRef, null)
    }

    override suspend fun clearToken() {
        SecItemDelete(baseQuery() as CFDictionaryRef)
    }

    private fun baseQuery(): MutableMap<Any?, Any?> =
        mutableMapOf(
            kSecClass to kSecClassGenericPassword,
            kSecAttrService to SERVICE,
            kSecAttrAccount to ACCOUNT,
        )

    private fun String.toNSData(): NSData? = (NSString.stringWithUTF8String(this) as NSString?)?.dataUsingEncoding(NSUTF8StringEncoding)

    private companion object {
        const val SERVICE = "com.poziomki.app.session"
        const val ACCOUNT = "session_token"
    }
}
