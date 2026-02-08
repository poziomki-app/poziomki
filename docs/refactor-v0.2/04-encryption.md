# Native Platform Encryption

## Why Native Crypto

| Aspect | JS Library (Noble) | Native Platform |
|--------|-------------------|-----------------|
| Key storage | Encrypted blob in SecureStorage | Hardware (Secure Enclave/StrongBox) |
| Key extraction | Possible (loaded into JS memory) | Impossible (hardware-bound) |
| Dependencies | +1 npm package | 0 (platform APIs) |
| Performance | JS runtime | Native code |
| Audit | Library audit | Platform audit (Apple/Google) |

## What is Secure Enclave / StrongBox?

**Secure Enclave (iOS):** A dedicated security coprocessor in Apple devices. It has its own encrypted memory and secure boot. Private keys generated in Secure Enclave never leave the hardware — all crypto operations happen inside the enclave.

**StrongBox (Android):** Hardware-backed Keystore using a dedicated secure processor (like a TPM). Keys are stored in tamper-resistant hardware. Even if the main OS is compromised, keys remain protected.

## Architecture

```
User A                              User B
   │                                   │
   ├── Generate P-256 keypair          ├── Generate P-256 keypair
   │   (hardware: Secure Enclave/      │   (hardware: Secure Enclave/
   │    StrongBox - never leaves)      │    StrongBox - never leaves)
   │                                   │
   └── Public key ─────────────────────┼── Exchange via server
                                       │
   ┌── Public key ─────────────────────┘
   │
   ▼
Shared Secret = ECDH(myPrivate, theirPublic)  ← computed in hardware
   │
   ▼
AES-256-GCM encrypt/decrypt  ← all in native code
   │
   ▼
Only ciphertext crosses to JS layer
```

## Native Module Interface

```typescript
// Native crypto module - all crypto happens in native code
declare module '@lynx-js/types' {
  interface NativeModules {
    Crypto: {
      // Key management (hardware-backed)
      generateKeyPair(tag: string): Promise<{ publicKey: string }>
      getPublicKey(tag: string): Promise<string | null>
      deleteKeyPair(tag: string): Promise<void>

      // ECDH key agreement (computed in hardware)
      deriveSharedSecret(myKeyTag: string, theirPublicKey: string): Promise<string>

      // AES-GCM encryption (native code)
      encrypt(secretTag: string, plaintext: string): Promise<{ ciphertext: string; nonce: string }>
      decrypt(secretTag: string, ciphertext: string, nonce: string): Promise<string>
    }
  }
}
```

## Android Implementation (Kotlin)

```kotlin
// NativeCryptoModule.kt
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties.*
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.spec.ECGenParameterSpec
import javax.crypto.Cipher
import javax.crypto.KeyAgreement
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

class NativeCryptoModule {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    fun generateKeyPair(tag: String): Map<String, String> {
        val spec = KeyGenParameterSpec.Builder(tag, PURPOSE_AGREE_KEY)
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            .setUserAuthenticationRequired(false)
            .setIsStrongBoxBacked(true) // Hardware security module
            .build()

        val keyPair = KeyPairGenerator.getInstance(KEY_ALGORITHM_EC, "AndroidKeyStore")
            .apply { initialize(spec) }
            .generateKeyPair()

        return mapOf("publicKey" to keyPair.public.encoded.toBase64())
    }

    fun deriveSharedSecret(myKeyTag: String, theirPublicKey: String): String {
        val myPrivateKey = keyStore.getKey(myKeyTag, null) as PrivateKey
        val theirKey = decodePublicKey(theirPublicKey)

        val sharedSecret = KeyAgreement.getInstance("ECDH").run {
            init(myPrivateKey)
            doPhase(theirKey, true)
            generateSecret()
        }

        // Store derived key for this conversation
        val secretTag = "shared_${myKeyTag}_${theirPublicKey.hashCode()}"
        storeSecretKey(secretTag, sharedSecret)
        return secretTag
    }

    fun encrypt(secretTag: String, plaintext: String): Map<String, String> {
        val secretKey = getSecretKey(secretTag)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.ENCRYPT_MODE, secretKey)
        }

        val ciphertext = cipher.doFinal(plaintext.toByteArray(Charsets.UTF_8))
        return mapOf(
            "ciphertext" to ciphertext.toBase64(),
            "nonce" to cipher.iv.toBase64()
        )
    }

    fun decrypt(secretTag: String, ciphertext: String, nonce: String): String {
        val secretKey = getSecretKey(secretTag)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.DECRYPT_MODE, secretKey, GCMParameterSpec(128, nonce.fromBase64()))
        }

        return String(cipher.doFinal(ciphertext.fromBase64()), Charsets.UTF_8)
    }
}
```

## iOS Implementation (Swift)

```swift
// NativeCryptoModule.swift
import CryptoKit
import Security

class NativeCryptoModule {

    func generateKeyPair(tag: String) throws -> [String: String] {
        // Generate key in Secure Enclave
        let privateKey = try SecureEnclave.P256.KeyAgreement.PrivateKey()

        // Store reference in Keychain
        let query: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: tag.data(using: .utf8)!,
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecValueRef as String: privateKey,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        SecItemAdd(query as CFDictionary, nil)

        return ["publicKey": privateKey.publicKey.rawRepresentation.base64EncodedString()]
    }

    func deriveSharedSecret(myKeyTag: String, theirPublicKey: String) throws -> String {
        let myPrivateKey = try getPrivateKey(tag: myKeyTag)
        let theirKey = try P256.KeyAgreement.PublicKey(
            rawRepresentation: Data(base64Encoded: theirPublicKey)!
        )

        let sharedSecret = try myPrivateKey.sharedSecretFromKeyAgreement(with: theirKey)

        // Derive symmetric key using HKDF
        let symmetricKey = sharedSecret.hkdfDerivedSymmetricKey(
            using: SHA256.self,
            salt: Data(),
            sharedInfo: "poziomki-chat-v1".data(using: .utf8)!,
            outputByteCount: 32
        )

        // Store for this conversation
        let secretTag = "shared_\(myKeyTag)_\(theirPublicKey.hashValue)"
        try storeSymmetricKey(symmetricKey, tag: secretTag)
        return secretTag
    }

    func encrypt(secretTag: String, plaintext: String) throws -> [String: String] {
        let key = try getSymmetricKey(tag: secretTag)
        let nonce = AES.GCM.Nonce()

        let sealedBox = try AES.GCM.seal(
            plaintext.data(using: .utf8)!,
            using: key,
            nonce: nonce
        )

        return [
            "ciphertext": sealedBox.ciphertext.base64EncodedString(),
            "nonce": Data(nonce).base64EncodedString()
        ]
    }

    func decrypt(secretTag: String, ciphertext: String, nonce: String) throws -> String {
        let key = try getSymmetricKey(tag: secretTag)

        let sealedBox = try AES.GCM.SealedBox(
            nonce: AES.GCM.Nonce(data: Data(base64Encoded: nonce)!),
            ciphertext: Data(base64Encoded: ciphertext)!,
            tag: Data()
        )

        let decrypted = try AES.GCM.open(sealedBox, using: key)
        return String(data: decrypted, encoding: .utf8)!
    }
}
```

## Usage in App

```typescript
// apps/mobile/src/hooks/useChatEncryption.ts

const MY_KEY_TAG = `chat_key_${getProfileId()}`

export function useChatEncryption(conversationId: string, theirPublicKey: string) {
  const [secretTag, setSecretTag] = useState<string | null>(null)

  // Initialize: generate keypair if needed, derive shared secret
  useEffect(() => {
    async function init() {
      // Ensure we have a keypair (hardware-backed)
      let myPublicKey = await NativeModules.Crypto.getPublicKey(MY_KEY_TAG)
      if (!myPublicKey) {
        const result = await NativeModules.Crypto.generateKeyPair(MY_KEY_TAG)
        myPublicKey = result.publicKey
        // Send public key to server for key exchange
        await api.profiles.me.publicKey.put({ body: { publicKey: myPublicKey } })
      }

      // Derive shared secret for this conversation
      const tag = await NativeModules.Crypto.deriveSharedSecret(MY_KEY_TAG, theirPublicKey)
      setSecretTag(tag)
    }
    init()
  }, [conversationId, theirPublicKey])

  return {
    isReady: secretTag !== null,

    encrypt: async (plaintext: string) => {
      if (!secretTag) throw new Error('Encryption not initialized')
      return NativeModules.Crypto.encrypt(secretTag, plaintext)
    },

    decrypt: async (ciphertext: string, nonce: string) => {
      if (!secretTag) throw new Error('Encryption not initialized')
      return NativeModules.Crypto.decrypt(secretTag, ciphertext, nonce)
    },
  }
}
```

## Key Exchange Flow

```
1. User A opens app for first time
   └── Native: generateKeyPair("chat_key_A") → hardware-backed
   └── API: PUT /profiles/me/publicKey { publicKey: "..." }

2. User A starts chat with User B
   └── API: GET /profiles/B → includes B's publicKey
   └── Native: deriveSharedSecret("chat_key_A", B.publicKey)
       └── ECDH computed in Secure Enclave / StrongBox
       └── Symmetric key stored in Keychain / Keystore

3. User A sends message
   └── Native: encrypt(secretTag, "Hello") → { ciphertext, nonce }
   └── API: POST /chats/{id}/messages { ciphertext, nonce }
   └── Server stores ciphertext (cannot decrypt)

4. User B receives message
   └── Native: decrypt(secretTag, ciphertext, nonce) → "Hello"
```

## Security Properties

| Property | Guarantee |
|----------|-----------|
| **Key isolation** | Private keys never leave secure hardware |
| **Forward secrecy** | Per-conversation derived keys |
| **Integrity** | AES-GCM provides authenticated encryption |
| **Server blindness** | Server only sees ciphertext, cannot decrypt |
| **Device binding** | Keys cannot be extracted or copied |

## Testing Crypto

```typescript
// Test vectors for encryption correctness
describe('NativeCrypto', () => {
  it('encrypt/decrypt round-trip', async () => {
    const { publicKey } = await NativeModules.Crypto.generateKeyPair('test-key')
    const secretTag = await NativeModules.Crypto.deriveSharedSecret('test-key', publicKey)

    const plaintext = 'Hello, World!'
    const { ciphertext, nonce } = await NativeModules.Crypto.encrypt(secretTag, plaintext)
    const decrypted = await NativeModules.Crypto.decrypt(secretTag, ciphertext, nonce)

    expect(decrypted).toBe(plaintext)
  })

  it('different nonces produce different ciphertext', async () => {
    // ... test randomness
  })

  it('wrong key fails to decrypt', async () => {
    // ... test integrity
  })
})
```
