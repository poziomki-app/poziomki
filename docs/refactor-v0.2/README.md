# Poziomki v0.2: LynxJS Rewrite

**Goal:** Simpler, more secure, performant, local-first app.

**Core Features:** Profiles + Chats + Events — nothing more.

**New Repo:** `poziomki-lynx`

## Design Principles

- **Dark mode only** — no light mode references or overhead
- **Gradient-first** — consistent visual language across all screens
- **Local-first chats** — offline-capable, sync when online
- **Same features, simpler code** — keep ALL features, reduce code 50%+

## Target Metrics

| Metric | Current | Target | Reduction |
|--------|---------|--------|-----------|
| Screens | 19 | 12 | 37% |
| Hooks | 42 | 15 | 64% |
| Components | 84 | 35 | 58% |
| Mobile LOC | ~17,500 | ~8,000 | 54% |
| API LOC | ~7,000 | ~4,500 | 36% |
| Dependencies | 903 | ~150 | 83% |
| APK Size | ~35 MB | ~18 MB | 49% |
| oxlint warnings | 101 | 0 | 100% |
| TypeScript errors | ? | 0 | 100% |
| Type assertions (`as`/`!`) | ~30 | 0 | 100% |
| Test coverage | 0% | 80%+ | ∞ |

## Documentation Structure

| Document | Description |
|----------|-------------|
| [01-architecture.md](./01-architecture.md) | Stack, screens, hooks, components |
| [02-design-system.md](./02-design-system.md) | Dark mode, gradients, CSS variables |
| [03-local-first-chat.md](./03-local-first-chat.md) | Electric SQL, TanStack DB, useChat |
| [04-encryption.md](./04-encryption.md) | Native crypto (Kotlin/Swift) |
| [05-infrastructure.md](./05-infrastructure.md) | Docker, SeaweedFS, Dragonfly |
| [06-development-practices.md](./06-development-practices.md) | oxlint, TypeScript, TDD |

## Migration Phases

| Phase | Weeks | Focus |
|-------|-------|-------|
| [Phase 1: Foundation](./phases/phase-1-foundation.md) | 1-2 | Setup + Auth |
| [Phase 2: Features](./phases/phase-2-features.md) | 3-4 | Profiles + Events + Infra |
| [Phase 3: Chat](./phases/phase-3-chat.md) | 5-6 | Chat + Encryption |
| [Phase 4: Release](./phases/phase-4-release.md) | 7-8 | Polish + Release |

## Checklists

- [Feature Parity](./checklists/feature-parity.md) — all features must work
- [Success Criteria](./checklists/success-criteria.md) — quality gates

## Decision Log

| Decision | Rationale |
|----------|-----------|
| **Same features, simpler code** | Keep ALL features, reduce code 50%+ |
| LynxJS over Expo | 6x fewer deps, smaller APK, dual-thread perf |
| TanStack Router | Type-safe, file-based, same ecosystem as Query |
| TanStack DB + Electric SQL | Local-first chat, offline support, instant UI |
| SeaweedFS over MinIO | MinIO in maintenance mode, Apache 2.0 license |
| Dragonfly for caching | 25x faster, pub/sub for typing, participant cache |
| Chainguard images | Near-zero CVEs, signed, SBOMs included |
| Native crypto over JS | Hardware-backed keys, no extraction possible |
| Dark mode only | Eliminates theme logic, consistent brand, simpler CSS |
| Gradient-first design | Consistent visual language, depth without images |
| Single `useChat()` hook | Replaces 14 hooks, all features in one place |
| Simple matching | Sort by shared tag count, no complex scoring |
| Single onboarding screen | Reduce friction, same fields |

## References

- [LynxJS Documentation](https://lynxjs.org/)
- [TanStack DB](https://tanstack.com/db/latest)
- [Electric SQL](https://electric-sql.com/docs/intro)
- [Chainguard Images](https://www.chainguard.dev/chainguard-images)
- [SeaweedFS](https://github.com/seaweedfs/seaweedfs)
- [Dragonfly](https://www.dragonflydb.io/docs)
