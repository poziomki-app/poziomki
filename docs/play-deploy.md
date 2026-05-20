# Play Store auto-deploy

Tag pushes (`vX.Y.Z`) trigger `.github/workflows/android-release.yml`, which
builds a signed AAB, creates a GitHub Release with it attached, and then
uploads the AAB to Play Console's **internal testing** track.

The upload step is gated on the `PLAY_SERVICE_ACCOUNT_JSON` repo secret. If
the secret is missing the step is skipped with a warning; the rest of the
release workflow still succeeds and the AAB is downloadable from the GitHub
Release.

## One-time setup

You only need to do this once. After it's done, every `vX.Y.Z` tag ships
to Play internal automatically.

### 1. Create the service account

1. Open [Google Cloud Console → IAM → Service Accounts](https://console.cloud.google.com/iam-admin/serviceaccounts)
   for the **same Google account** that owns the Play Console listing.
2. **Create service account** → name it e.g. `play-publisher`. Skip the
   optional role grants in step 2 (Play permissions live in Play Console,
   not GCP). Click **Done**.
3. Click into the new service account → **Keys** tab → **Add key → Create
   new key → JSON**. A JSON file downloads. Keep it safe — it's the
   credential the workflow will use.

### 2. Grant Play Console access

1. Open [Play Console → Users and permissions](https://play.google.com/console/users-and-permissions).
2. **Invite new users** → paste the service account's email
   (`play-publisher@<project>.iam.gserviceaccount.com`).
3. **App permissions** → add `Poziomki` (app.poziomki) → grant at minimum:
   - **Release to testing tracks** (covers internal/closed/open)
   - **View app information and download bulk reports**
4. Save. The grant is effective immediately; no email confirmation needed.

### 3. Store the secret

```bash
gh secret set PLAY_SERVICE_ACCOUNT_JSON --repo poziomki-app/poziomki < path/to/play-publisher-XXXX.json
```

(or paste the raw JSON in GitHub → Settings → Secrets and variables → Actions
→ New repository secret, name `PLAY_SERVICE_ACCOUNT_JSON`).

### 4. Bootstrap the first Play release manually

Google's API refuses to publish if the app has never had a release on the
target track. So the **first AAB** must be uploaded by hand:

1. Build a tag (`git tag vX.Y.Z && git push origin vX.Y.Z`) — workflow will
   upload to GitHub Releases but skip Play (no secret yet, or step will
   fail with "no existing release" if secret was added before bootstrap).
2. Download the AAB from the GitHub Release.
3. Play Console → Testing → Internal testing → **Create new release** →
   upload the AAB → fill release notes → **Save → Review → Start rollout**.

From the next tag onward the workflow upload step takes over.

## Troubleshooting

- **403 / PERMISSION_DENIED on `androidpublisher.edits.insert`**: the
  service account is not granted on the app. Re-check step 2.
- **400 / `apkNotificationMessageKeyUpgradeVersionConflict`**: the AAB's
  `versionCode` is ≤ the latest version on Play. Bump `appVersionName` in
  `mobile/androidApp/build.gradle.kts`; the code is derived from it.
- **`Package not found: app.poziomki`**: the bootstrap upload (step 4) was
  skipped. Do it once manually.
- **`Track 'internal' is not properly configured`**: same as above.
- The Play upload step is silently skipped if `PLAY_SERVICE_ACCOUNT_JSON`
  isn't set. The workflow log shows a `::warning::` line — that's normal
  pre-bootstrap.
