# iOS TestFlight auto-deploy

Pushing a `v*` tag triggers `.github/workflows/ios-release.yml`, which builds the
iOS app on the self-hosted Mac mini runner and uploads it to TestFlight via
fastlane. Manual `workflow_dispatch` is supported too.

## One-time setup

### 1. App Store Connect API key

1. Go to <https://appstoreconnect.apple.com> → **Users and Access** →
   **Integrations** → **App Store Connect API** → **Team Keys**.
2. Generate a key with **App Manager** role (Admin also works).
3. Note the **Issuer ID**, **Key ID**, and download the `.p8` file (one-time
   download — keep it safe).
4. Base64-encode the `.p8`:

   ```bash
   base64 -i AuthKey_XXXXXXXXXX.p8 | tr -d '\n' | pbcopy
   ```

### 2. fastlane match private repo

`match` stores the encrypted distribution certificate and provisioning profile
in `poziomki-app/certs` (private). The Mac mini runner authenticates via SSH
deploy key (already configured at `~/.ssh/poziomki_certs` with a `github-certs`
host alias).

Bootstrap (one time, on the Mac mini):

```bash
cd ~/poziomki/mobile/iosApp   # or wherever the repo is checked out
bundle install
export MATCH_GIT_URL=git@github-certs:poziomki-app/certs.git
export MATCH_PASSWORD='choose-a-strong-passphrase'
export APPLE_TEAM_ID=ABCDE12345
export IOS_BUNDLE_ID=app.poziomki.ios
export ASC_KEY_ID=XXXXXXXXXX
export ASC_ISSUER_ID=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
export ASC_KEY_P8="$(base64 -i ~/AuthKey_XXXXXXXXXX.p8 | tr -d '\n')"
bundle exec fastlane match appstore
```

This creates the distribution cert + App Store provisioning profile and
commits them (encrypted) to the certs repo. After this, CI runs in `readonly`
mode and never modifies the repo.

### 3. GitHub repo secrets

Add the following at **Settings → Secrets and variables → Actions**:

| Secret | Value |
|---|---|
| `APPLE_TEAM_ID` | 10-char team ID from <https://developer.apple.com/account> → Membership |
| `ASC_KEY_ID` | App Store Connect API Key ID |
| `ASC_ISSUER_ID` | App Store Connect API Issuer ID |
| `ASC_KEY_P8_B64` | base64 of the `.p8` file (single line, no newlines) |
| `MATCH_PASSWORD` | the passphrase used to encrypt the match repo |
| `IOS_GOOGLE_SERVICE_INFO_PLIST_B64` | base64 of the real `GoogleService-Info.plist` |

### 4. Self-hosted runner on the Mac mini

1. In the repo: **Settings → Actions → Runners → New self-hosted runner**,
   select macOS / arm64. Copy the registration token.
2. On `macmini`:

   ```bash
   mkdir -p ~/actions-runner && cd ~/actions-runner
   curl -O -L https://github.com/actions/runner/releases/download/v2.319.1/actions-runner-osx-arm64-2.319.1.tar.gz
   tar xzf ./actions-runner-osx-arm64-2.319.1.tar.gz
   ./config.sh --url https://github.com/<owner>/<repo> --token <token> --labels self-hosted,macOS,arm64
   ./svc.sh install
   ./svc.sh start
   ```

3. Install fastlane prerequisites once:

   ```bash
   /opt/homebrew/bin/brew install rbenv ruby-build
   rbenv install 3.2.9 && rbenv global 3.2.9
   gem install bundler
   ```

## Releasing

Bump `MARKETING_VERSION` in `mobile/iosApp/Configuration/Config.xcconfig`,
commit, then tag:

```bash
git tag v0.21.5
git push origin v0.21.5
```

The workflow validates the tag matches `MARKETING_VERSION`, builds, and
uploads. `CURRENT_PROJECT_VERSION` (TestFlight build number) is auto-derived
from the current timestamp so it always monotonically increases.

After upload, TestFlight needs ~10–30 minutes to process the build before it's
testable.
