# Changelog

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
* **search,events:** filter blocked users from search results and attendee list ([#276](https://github.com/poziomki-app/poziomki/issues/276)) ([e221e85](https://github.com/poziomki-app/poziomki/commit/e221e8556018cd46e30422bee85ba7d346710358))
