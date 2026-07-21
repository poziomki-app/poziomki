# Changelog

## [0.20.0](https://github.com/poziomki-app/poziomki/compare/v0.19.0...v0.20.0) (2026-07-21)


### Features

* add optional event attendee limits ([#57](https://github.com/poziomki-app/poziomki/issues/57)) ([26968e6](https://github.com/poziomki-app/poziomki/commit/26968e6fd8c888c80f9176a22a5a3f4acf0d7440))
* **backend:** dev-only endpoints for e2e tests ([#446](https://github.com/poziomki-app/poziomki/issues/446)) ([5ae572b](https://github.com/poziomki-app/poziomki/commit/5ae572b3587b8451a34e747a5cc35019e3e4dfc7))
* **backend:** sentry error and perf reporting ([#480](https://github.com/poziomki-app/poziomki/issues/480)) ([36366bf](https://github.com/poziomki-app/poziomki/commit/36366bffecc39c4b486761fd1f00c051dab02139))
* **chat:** gaussian blur + Zgłoś chip on flagged messages ([#271](https://github.com/poziomki-app/poziomki/issues/271)) ([ca9f7fc](https://github.com/poziomki-app/poziomki/commit/ca9f7fcabf415c1aee8f702bbd613493ac8925d2))
* **chat:** per-message report endpoint + nicer blur ([#277](https://github.com/poziomki-app/poziomki/issues/277)) ([8adcbb3](https://github.com/poziomki-app/poziomki/commit/8adcbb3dd5f1f115377c7437486f8f88998c5a2b))
* **chat:** real indicators — receipts, delivery, mute, typing TTL ([#290](https://github.com/poziomki-app/poziomki/issues/290)) ([9fe8245](https://github.com/poziomki-app/poziomki/commit/9fe8245cef7bbcf614c0109117c7ab0d42e76b0f))
* event interactions, tag management, and save/unsave ([#69](https://github.com/poziomki-app/poziomki/issues/69)) ([21c9c5b](https://github.com/poziomki-app/poziomki/commit/21c9c5b9f2ca6d7d28be9597ca8bc45c07dd8363))
* event join requiring approval ([#44](https://github.com/poziomki-app/poziomki/issues/44)) ([#66](https://github.com/poziomki-app/poziomki/issues/66)) ([19ac0cd](https://github.com/poziomki-app/poziomki/commit/19ac0cd357bf25f5ee7c80213aa1c5c46b746b40))
* event recommendation schema foundation ([#61](https://github.com/poziomki-app/poziomki/issues/61)) ([7bd1cec](https://github.com/poziomki-app/poziomki/commit/7bd1cecc0943d1ee092023207e045367828272ce))
* **events:** admin-only featured flag + wyróżnione badge ([#509](https://github.com/poziomki-app/poziomki/issues/509)) ([c174d94](https://github.com/poziomki-app/poziomki/commit/c174d94652b3ce4eec3d67e02d1708b46dae59d2))
* **feedback:** feature-request field + email to kontakt@poziomki.app ([#499](https://github.com/poziomki-app/poziomki/issues/499)) ([e06c709](https://github.com/poziomki-app/poziomki/commit/e06c709eddb3f26bab0abc215f928f5c9b7a9ec2))
* **feedback:** welcome modal + in-app feedback for test phase ([#454](https://github.com/poziomki-app/poziomki/issues/454)) ([be5a929](https://github.com/poziomki-app/poziomki/commit/be5a929332a9e4ee0f103b2b04cdc8364d0529d9))
* gamification — XP, streaks, QR meet-ups, daily tasks ([661852c](https://github.com/poziomki-app/poziomki/commit/661852cc6d1b38c4d169739b58eb001cb9c807cd))
* **moderation:** blur flagged chat messages, sync-block events + bios ([#267](https://github.com/poziomki-app/poziomki/issues/267)) ([845b3d9](https://github.com/poziomki-app/poziomki/commit/845b3d9b84669d6987b994e27ab87b47094c6fda))
* **moderation:** NSFW gate for user uploads via Marqo ([#327](https://github.com/poziomki-app/poziomki/issues/327)) ([4b3ddc6](https://github.com/poziomki-app/poziomki/commit/4b3ddc654e4e81e3938381ad5d306d17d2208698))
* **moderation:** Polish content safety via Bielik-Guard int8 ONNX ([#257](https://github.com/poziomki-app/poziomki/issues/257)) ([2441c22](https://github.com/poziomki-app/poziomki/commit/2441c22ddc1dbc02faab15cbc0d9a402085073ac))
* **observability:** finish Sentry + Crashlytics E2E ([#505](https://github.com/poziomki-app/poziomki/issues/505)) ([4f724c0](https://github.com/poziomki-app/poziomki/commit/4f724c072049bc35c7b231840a807d4fc36d0d6e))
* **otp:** add Resend HTTP fallback for OTP delivery ([#382](https://github.com/poziomki-app/poziomki/issues/382)) ([b97e4c6](https://github.com/poziomki-app/poziomki/commit/b97e4c6a692780b63b1e6947b65a507212943004))
* powiadomienia settings + per-conversation mute ([#412](https://github.com/poziomki-app/poziomki/issues/412)) ([7687ec8](https://github.com/poziomki-app/poziomki/commit/7687ec81b1fe87f93975ff04ec62fc254724363e))
* **privacy:** change email with OTP ([#381](https://github.com/poziomki-app/poziomki/issues/381)) ([0d8fd0c](https://github.com/poziomki-app/poziomki/commit/0d8fd0cc414287b2a8297a6582d9fb8f0f8ce0d5))
* **push:** admin broadcast notifications ([#492](https://github.com/poziomki-app/poziomki/issues/492)) ([60dc76b](https://github.com/poziomki-app/poziomki/commit/60dc76bfa320111a7ec79a5adaa4e2d92c145787))
* **push:** notify on tag-matched new events ([#496](https://github.com/poziomki-app/poziomki/issues/496)) ([2115ce5](https://github.com/poziomki-app/poziomki/commit/2115ce51f186b5862a5bb65843452592406e9949))
* replace ntfy push with FCM data-only wake-ups ([#403](https://github.com/poziomki-app/poziomki/issues/403)) ([a173b53](https://github.com/poziomki-app/poziomki/commit/a173b53d006c114c47eb5d3ed1ae2151fa3ce38f))
* **review:** is_review_stub flag to isolate reviewer and test data ([#165](https://github.com/poziomki-app/poziomki/issues/165)) ([3e99568](https://github.com/poziomki-app/poziomki/commit/3e99568afad30d3b711bcb0ffe28afeada3e0559))
* self-host OSRM walking routes, drop public demo ([#404](https://github.com/poziomki-app/poziomki/issues/404)) ([7439623](https://github.com/poziomki-app/poziomki/commit/743962356cbd2d65360d0ee2287f77f61306891d))
* **staging:** proper role isolation on slim parallel stack ([#253](https://github.com/poziomki-app/poziomki/issues/253)) ([0105152](https://github.com/poziomki-app/poziomki/commit/0105152321cb0f76ed49991e1e763978cdec1165))
* tag-based event recommendation scoring ([#70](https://github.com/poziomki-app/poziomki/issues/70)) ([a583338](https://github.com/poziomki-app/poziomki/commit/a58333850ffb57e53a157d800b59f0222e079a7c))


### Bug Fixes

* **api:** wrap delete_account response so client parses as Success ([#433](https://github.com/poziomki-app/poziomki/issues/433)) ([fd3ada7](https://github.com/poziomki-app/poziomki/commit/fd3ada7ec13511f89d63c30aafd5e537989e1bf3))
* atomic attendee+interaction writes, gitignore cleanup ([59c7c2e](https://github.com/poziomki-app/poziomki/commit/59c7c2e000c37f091d9a81f1ad0eb8628381f32e))
* **auth:** wrong password on delete/change doesn't log out ([#428](https://github.com/poziomki-app/poziomki/issues/428)) ([eb54fa5](https://github.com/poziomki-app/poziomki/commit/eb54fa545740183aa6c4b91cf24e98dae745c6c3))
* backend security and logic issues from code review ([#72](https://github.com/poziomki-app/poziomki/issues/72)) ([b5ea7cd](https://github.com/poziomki-app/poziomki/commit/b5ea7cd1ae59cfed2b86517a4d0da6f32842a4a6))
* **backend:** bump diesel and cmov to patched versions ([#595](https://github.com/poziomki-app/poziomki/issues/595)) ([14b4229](https://github.com/poziomki-app/poziomki/commit/14b4229c3cbb51edf12c3d3619ed260477323f47))
* **backend:** filter blocked profiles from bookmark list ([#287](https://github.com/poziomki-app/poziomki/issues/287)) ([8ec250d](https://github.com/poziomki-app/poziomki/commit/8ec250d5d254cb0bf4e4bf664ec2b2b90d20d702))
* **backend:** install bash before SHELL pipefail directive ([#231](https://github.com/poziomki-app/poziomki/issues/231)) ([a210174](https://github.com/poziomki-app/poziomki/commit/a210174ac45c3e4ffeaf2244f23187b03dd99a1f))
* **backend:** resolve cargo-deny advisory failures blocking CI ([#598](https://github.com/poziomki-app/poziomki/issues/598)) ([3e1d8bf](https://github.com/poziomki-app/poziomki/commit/3e1d8bf99c4230ada0670d697ae7ad56e421510b))
* **backend:** verify upload ownership for profile images and event covers ([#288](https://github.com/poziomki-app/poziomki/issues/288)) ([1dec5e2](https://github.com/poziomki-app/poziomki/commit/1dec5e2684b03dfe6b9f4c9f36f3e1071c99380a))
* bound catalog search input, exclude test paths from CodeQL ([#33](https://github.com/poziomki-app/poziomki/issues/33)) ([207c49d](https://github.com/poziomki-app/poziomki/commit/207c49d089377681c96d05373a005b48f11870f6))
* **chat:** allow api.poziomki.app as WS origin ([#168](https://github.com/poziomki-app/poziomki/issues/168)) ([e7dd49b](https://github.com/poziomki-app/poziomki/commit/e7dd49bac3c85ada1e2b18f6cd4a55e7e52d2895))
* **chat:** cap message reports at 30 per reporter per 24h ([#338](https://github.com/poziomki-app/poziomki/issues/338)) ([0e149c7](https://github.com/poziomki-app/poziomki/commit/0e149c7704a0eff95b0c7b271426b6edc4a748d2))
* **chat:** close message-id enumeration in reveal handler ([#278](https://github.com/poziomki-app/poziomki/issues/278)) ([c397a49](https://github.com/poziomki-app/poziomki/commit/c397a496125189b5f327590ae2e9e3be7da5931a))
* **chat:** include moderation_* columns in latest-message query ([#269](https://github.com/poziomki-app/poziomki/issues/269)) ([b7ca64e](https://github.com/poziomki-app/poziomki/commit/b7ca64e6a203b452a0b37c18fbcda1d8dcf5031f))
* **chat:** redact flagged latest-message in conversation list preview ([#270](https://github.com/poziomki-app/poziomki/issues/270)) ([3ec1beb](https://github.com/poziomki-app/poziomki/commit/3ec1beb0bd03f8da2f169cbf78b21fb496db344c))
* **chat:** report envelope + hidden-message UI + auto-picked reason ([#284](https://github.com/poziomki-app/poziomki/issues/284)) ([d5c887a](https://github.com/poziomki-app/poziomki/commit/d5c887a56569dfc04621c63016ebddd63c8143d3))
* **chat:** RLS rejects first DM/event chat insert ([#294](https://github.com/poziomki-app/poziomki/issues/294)) ([19e60f6](https://github.com/poziomki-app/poziomki/commit/19e60f65f2a8fcc060698adbe73581ab0aee9f93))
* close auth and client token security gaps ([3d9f423](https://github.com/poziomki-app/poziomki/commit/3d9f423fc27bb5ea96c47bc72f9802e24cc73fc5))
* **deploy:** stage libs under /usr to unbreak buildkit on trixie ([#258](https://github.com/poziomki-app/poziomki/issues/258)) ([aa47183](https://github.com/poziomki-app/poziomki/commit/aa4718345070a7c4e4786dae3d8876c830e6e23d))
* Dockerfile WORKDIR for repo-root build context, bump mobile to v0.15 ([6cde3ee](https://github.com/poziomki-app/poziomki/commit/6cde3ee7eb0db1ef6b5e95340be3c8bdcccdaffe))
* **events:** hide finished events from listings and cache ([#274](https://github.com/poziomki-app/poziomki/issues/274)) ([9c84ac8](https://github.com/poziomki-app/poziomki/commit/9c84ac8e3f5e9a922936bc75a6d283cc851b2cca))
* **fcm:** enable jsonwebtoken rust_crypto feature ([#405](https://github.com/poziomki-app/poziomki/issues/405)) ([7bf6362](https://github.com/poziomki-app/poziomki/commit/7bf6362f6c3f99a8a189bc204092ce36460d935f))
* **feedback:** RLS-aware insert + scrollable dialog ([#503](https://github.com/poziomki-app/poziomki/issues/503)) ([d7ea378](https://github.com/poziomki-app/poziomki/commit/d7ea37857fcea446d71de35a6b177a196552c916))
* **ios:** App Store compliance — legal gate, moderation, account deletion, portrait lock ([#573](https://github.com/poziomki-app/poziomki/issues/573)) ([e7b9f1a](https://github.com/poziomki-app/poziomki/commit/e7b9f1a3d619646adb0047c4e0205b327f86a35e))
* **metrics:** bind exporter to 0.0.0.0 inside container ([#411](https://github.com/poziomki-app/poziomki/issues/411)) ([1ff2dde](https://github.com/poziomki-app/poziomki/commit/1ff2dde499905244da9420d87fc0dcb52956c880))
* **notifications:** shorten label, default to DMs only ([#436](https://github.com/poziomki-app/poziomki/issues/436)) ([eff9b16](https://github.com/poziomki-app/poziomki/commit/eff9b16358c3f8716bdd91021a2301532e97121b))
* **onboarding:** footer lifts above keyboard ([#434](https://github.com/poziomki-app/poziomki/issues/434)) ([89fd3dc](https://github.com/poziomki-app/poziomki/commit/89fd3dceb4cb275cac668a30985c4ed2db816172))
* **otp:** SMTP relay via Lettre ([#295](https://github.com/poziomki-app/poziomki/issues/295)) ([8bdee48](https://github.com/poziomki-app/poziomki/commit/8bdee485c815114dcaeeec9d2e185376967ab4fc))
* picker layout + polecane pins featured first ([#513](https://github.com/poziomki-app/poziomki/issues/513)) ([37ff8c7](https://github.com/poziomki-app/poziomki/commit/37ff8c7d8633f7be697f702b2d9b5eba3bf1d0b1))
* **polecane:** keep joined events, show check badge ([#502](https://github.com/poziomki-app/poziomki/issues/502)) ([d91d507](https://github.com/poziomki-app/poziomki/commit/d91d507208800a4c440f9eda10a67c1ed40a72e5))
* **push:** actually delete stale FCM tokens ([#493](https://github.com/poziomki-app/poziomki/issues/493)) ([003cee6](https://github.com/poziomki-app/poziomki/commit/003cee6c63c957f65b6d491d665b4bad7f09ec13))
* **push:** make Android notifications actually fire ([#489](https://github.com/poziomki-app/poziomki/issues/489)) ([531f2ad](https://github.com/poziomki-app/poziomki/commit/531f2add814da223d986d9c8528a489ec9a1c273))
* remove .expect() on TestServer::new() ([#26](https://github.com/poziomki-app/poziomki/issues/26)) ([a639080](https://github.com/poziomki-app/poziomki/commit/a6390804ffe51c40aa9a10cf7e090af6fd48cc2b))
* **rls:** allow NULL'ing own upload owner_id ([#431](https://github.com/poziomki-app/poziomki/issues/431)) ([c681f95](https://github.com/poziomki-app/poziomki/commit/c681f9554cb14dda4486609c5d7251d5d617a696))
* **rls:** let viewer SELECT own profile so INSERT...RETURNING works ([#425](https://github.com/poziomki-app/poziomki/issues/425)) ([597b284](https://github.com/poziomki-app/poziomki/commit/597b284ddd9dc204688a41a8307a0885a49ebf0e))
* **rls:** materialise app.* GUCs via set_config (PG18 regression) ([#385](https://github.com/poziomki-app/poziomki/issues/385)) ([9bb077e](https://github.com/poziomki-app/poziomki/commit/9bb077efe46db3ca117dd194256968f285bbff8b))
* **rls:** split set_config into separate statements ([#387](https://github.com/poziomki-app/poziomki/issues/387)) ([70f3746](https://github.com/poziomki-app/poziomki/commit/70f3746a5531610d38301cd7fdaa4c22364ebb87))
* **rls:** viewer can DELETE own DM conversations (account-delete) ([#432](https://github.com/poziomki-app/poziomki/issues/432)) ([d6e47e8](https://github.com/poziomki-app/poziomki/commit/d6e47e8a2cd884902f042b83b6f596ee472f8e31))
* screenshot default, delete-account, keep session on pw change ([#427](https://github.com/poziomki-app/poziomki/issues/427)) ([3cc7ae9](https://github.com/poziomki-app/poziomki/commit/3cc7ae915d59665a5bb2ef7421123ebabcd48025))
* **search,events:** filter blocked users from search results and attendee list ([#276](https://github.com/poziomki-app/poziomki/issues/276)) ([e221e85](https://github.com/poziomki-app/poziomki/commit/e221e8556018cd46e30422bee85ba7d346710358))
* **security:** tagIds allocation guard ([#138](https://github.com/poziomki-app/poziomki/issues/138)) ([32bc1d4](https://github.com/poziomki-app/poziomki/commit/32bc1d43c272a818cd2613376f34ff09e9a3b5be))
* **uploads:** fail-closed image moderation when engine errors ([#334](https://github.com/poziomki-app/poziomki/issues/334)) ([f130304](https://github.com/poziomki-app/poziomki/commit/f1303044f9bf6bba0932407bdb5d9a2e32983bbc))
* wrap XP endpoint responses in DataResponse envelope ([45f8524](https://github.com/poziomki-app/poziomki/commit/45f85247b09e437baf97c239b0319ac254943bb6))
* **xp:** atomic award for both sides on QR scan ([#302](https://github.com/poziomki-app/poziomki/issues/302)) ([224fdf5](https://github.com/poziomki-app/poziomki/commit/224fdf54899a3eab12911dd0a43034bf1d50dfcd))


### Performance

* **auth:** cache /auth/get-session through the auth cache ([#417](https://github.com/poziomki-app/poziomki/issues/417)) ([f142dec](https://github.com/poziomki-app/poziomki/commit/f142dec8b80ae4c77fb993f188f5cc4bc8c38b43))
* **matching:** cache tag-parent map with 60s TTL ([#415](https://github.com/poziomki-app/poziomki/issues/415)) ([8a67f39](https://github.com/poziomki-app/poziomki/commit/8a67f395e8ba63827b68d689ee79676b0208eac4))


### Reverts

* **chat:** roll back indicators [#290](https://github.com/poziomki-app/poziomki/issues/290), keep typing only ([#296](https://github.com/poziomki-app/poziomki/issues/296)) ([e3151cd](https://github.com/poziomki-app/poziomki/commit/e3151cd52972b0ae64077443501b331923dbffdf))

## [0.19.0](https://github.com/poziomki-app/poziomki/compare/v0.18.4...v0.19.0) (2026-04-28)


### Features

* **chat:** gaussian blur + Zgłoś chip on flagged messages ([#271](https://github.com/poziomki-app/poziomki/issues/271)) ([ca9f7fc](https://github.com/poziomki-app/poziomki/commit/ca9f7fcabf415c1aee8f702bbd613493ac8925d2))
* **chat:** per-message report endpoint + nicer blur ([#277](https://github.com/poziomki-app/poziomki/issues/277)) ([8adcbb3](https://github.com/poziomki-app/poziomki/commit/8adcbb3dd5f1f115377c7437486f8f88998c5a2b))
* **chat:** real indicators — receipts, delivery, mute, typing TTL ([#290](https://github.com/poziomki-app/poziomki/issues/290)) ([9fe8245](https://github.com/poziomki-app/poziomki/commit/9fe8245cef7bbcf614c0109117c7ab0d42e76b0f))
* **moderation:** blur flagged chat messages, sync-block events + bios ([#267](https://github.com/poziomki-app/poziomki/issues/267)) ([845b3d9](https://github.com/poziomki-app/poziomki/commit/845b3d9b84669d6987b994e27ab87b47094c6fda))
* **moderation:** Polish content safety via Bielik-Guard int8 ONNX ([#257](https://github.com/poziomki-app/poziomki/issues/257)) ([2441c22](https://github.com/poziomki-app/poziomki/commit/2441c22ddc1dbc02faab15cbc0d9a402085073ac))
* **staging:** proper role isolation on slim parallel stack ([#253](https://github.com/poziomki-app/poziomki/issues/253)) ([0105152](https://github.com/poziomki-app/poziomki/commit/0105152321cb0f76ed49991e1e763978cdec1165))


### Bug Fixes

* **backend:** filter blocked profiles from bookmark list ([#287](https://github.com/poziomki-app/poziomki/issues/287)) ([8ec250d](https://github.com/poziomki-app/poziomki/commit/8ec250d5d254cb0bf4e4bf664ec2b2b90d20d702))
* **backend:** verify upload ownership for profile images and event covers ([#288](https://github.com/poziomki-app/poziomki/issues/288)) ([1dec5e2](https://github.com/poziomki-app/poziomki/commit/1dec5e2684b03dfe6b9f4c9f36f3e1071c99380a))
* **chat:** close message-id enumeration in reveal handler ([#278](https://github.com/poziomki-app/poziomki/issues/278)) ([c397a49](https://github.com/poziomki-app/poziomki/commit/c397a496125189b5f327590ae2e9e3be7da5931a))
* **chat:** include moderation_* columns in latest-message query ([#269](https://github.com/poziomki-app/poziomki/issues/269)) ([b7ca64e](https://github.com/poziomki-app/poziomki/commit/b7ca64e6a203b452a0b37c18fbcda1d8dcf5031f))
* **chat:** redact flagged latest-message in conversation list preview ([#270](https://github.com/poziomki-app/poziomki/issues/270)) ([3ec1beb](https://github.com/poziomki-app/poziomki/commit/3ec1beb0bd03f8da2f169cbf78b21fb496db344c))
* **chat:** report envelope + hidden-message UI + auto-picked reason ([#284](https://github.com/poziomki-app/poziomki/issues/284)) ([d5c887a](https://github.com/poziomki-app/poziomki/commit/d5c887a56569dfc04621c63016ebddd63c8143d3))
* close auth and client token security gaps ([3d9f423](https://github.com/poziomki-app/poziomki/commit/3d9f423fc27bb5ea96c47bc72f9802e24cc73fc5))
* **deploy:** stage libs under /usr to unbreak buildkit on trixie ([#258](https://github.com/poziomki-app/poziomki/issues/258)) ([aa47183](https://github.com/poziomki-app/poziomki/commit/aa4718345070a7c4e4786dae3d8876c830e6e23d))
* **events:** hide finished events from listings and cache ([#274](https://github.com/poziomki-app/poziomki/issues/274)) ([9c84ac8](https://github.com/poziomki-app/poziomki/commit/9c84ac8e3f5e9a922936bc75a6d283cc851b2cca))
* **otp:** SMTP relay via Lettre ([#295](https://github.com/poziomki-app/poziomki/issues/295)) ([8bdee48](https://github.com/poziomki-app/poziomki/commit/8bdee485c815114dcaeeec9d2e185376967ab4fc))
* **search,events:** filter blocked users from search results and attendee list ([#276](https://github.com/poziomki-app/poziomki/issues/276)) ([e221e85](https://github.com/poziomki-app/poziomki/commit/e221e8556018cd46e30422bee85ba7d346710358))
* **xp:** atomic award for both sides on QR scan ([#302](https://github.com/poziomki-app/poziomki/issues/302)) ([224fdf5](https://github.com/poziomki-app/poziomki/commit/224fdf54899a3eab12911dd0a43034bf1d50dfcd))


### Reverts

* **chat:** roll back indicators [#290](https://github.com/poziomki-app/poziomki/issues/290), keep typing only ([#296](https://github.com/poziomki-app/poziomki/issues/296)) ([e3151cd](https://github.com/poziomki-app/poziomki/commit/e3151cd52972b0ae64077443501b331923dbffdf))
