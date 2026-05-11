# Ecosystem — Manga Server / Online Source Landscape

## Position

This project is a **local-first native reader**. Online sources are out of scope for MVP. Phase 6 adds OPDS client capability — we become a client of existing manga server ecosystems, **we do not build a competing online source system**.

## Why we do not run Mihon extensions directly

[Mihon](https://mihon.app/) (formerly Tachiyomi) is an Android manga reader with a massive extension ecosystem — extensions are APKs containing Kotlin/DEX bytecode implementing scraping interfaces.

To run them in our app would require:
- JVM (or JVM-compatible bytecode interpreter)
- AndroidCompat shim (mimicking Android `Context`, `SharedPreferences`, etc.)
- APK-to-JAR conversion + bytecode patching

This work has been **done by Suwayomi** for years. Replicating it is self-defeating.

## Suwayomi-Server — the bridge

[Suwayomi-Server](https://github.com/Suwayomi/Suwayomi-Server) (formerly Tachidesk) runs unmodified Mihon extensions on JVM via its AndroidCompat layer, and exposes them over:

- **GraphQL** (preferred for new clients)
- **REST**
- **OPDS** (interoperable with any e-book reader)

Active as of 2026. Multiple existing clients confirm API stability.

## OPDS — the wider abstraction

OPDS (Open Publication Distribution System) is a generic catalog protocol over HTTP + Atom/JSON. Supporting **OPDS client** in our reader gets us four ecosystems at once:

| Server | Stack | Position |
|---|---|---|
| **Suwayomi** | JVM | Bridge to Mihon online sources |
| **Komga** | Java | Popular self-hosted manga library |
| **Kavita** | .NET | Popular self-hosted manga/book library |
| **calibre-web** | Python | E-book mainstream (manga support partial) |

**One client implementation, four server ecosystems.** This is dramatically more leverage than implementing Suwayomi-specific GraphQL.

## Architectural implication (already in plan)

`PageSource` is async from day 1, even though MVP only has local sync sources. When Phase 6 lands `RemoteOpdsSource`, it slots into the same trait. Reader and UI code do not change.

## Why not just be a Suwayomi client?

We could fast-path Suwayomi via GraphQL and ignore the wider OPDS picture. Reasons we won't:

- OPDS is standardised; GraphQL schema is per-server
- Users running Komga / Kavita today shouldn't be forced to also run Suwayomi
- OPDS effort is bounded; GraphQL means tracking schema changes per release

If a user wants Mihon-extension content specifically, they install Suwayomi-Server themselves and point us at its OPDS endpoint.

## Out of scope (explicitly)

- Direct manga site scrapers built into our binary
- Maintaining or curating online source plug-ins
- Tracker integration (MAL, AniList, Kitsu) — defer to Phase 7+ if ever
- Cross-device sync of reading progress

## Sources

- [Mihon](https://mihon.app/)
- [Suwayomi-Server](https://github.com/Suwayomi/Suwayomi-Server)
- [Suwayomi Extension System (DeepWiki)](https://deepwiki.com/Suwayomi/Suwayomi-Server/4-extension-system)
- [Komga](https://komga.org/)
- [Kavita](https://www.kavitareader.com/)
- [OPDS Catalog Specification 1.2](https://specs.opds.io/opds-1.2)
- [OPDS Catalog Specification 2.0](https://drafts.opds.io/opds-2.0)
