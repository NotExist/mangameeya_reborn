# Original マンガミーヤ — Research Notes

## What it was

Windows-only manga reader, widely beloved in Japanese / CJK manga community for **speed** and **keyboard-driven UX**. Last known active development paused; current Windows community still uses it (2025 5ch threads active).

Variants:
- **MangaMeeya** — desktop Windows
- **MangaMeeyaCE** — Windows Mobile / WinCE (smaller subset)

## Why it's worth reviving

- Subjective page-turn latency that newer readers (e.g. CDisplayEx, Honeyview, MangaMeeya-clones) haven't matched
- Plug-in ecosystem (Susie .spi) means it can handle obscure archive / image formats
- Keyboard binding fully customisable via INI
- Single-folder install, fully portable

## Confirmed behaviours / facts

### Plug-in architecture
- Uses **Susie plug-in** format (`.spi` DLLs)
- Plug-ins dropped into `SusiePlugin/` subfolder relative to exe
- Two types observed: image format plug-ins (`00IN`) and archive plug-ins (`00AM`)
- RAR: via `unrar.spi` (RAR extract plug-in — own implementation, no external lib)
- 7z: via `ax7z.spi` (covers 7z, RAR, ZIP, LZH; needs 32-bit 7z.dll ≥ 15.07 for RAR5)
- MangaMeeya is 32-bit, so all plug-ins must be 32-bit
- 註：Susie compat is not perfect — some plug-ins cause CPU spikes or crashes per community reports

### Configuration
- File: `MangaMeeyaCE.ini` (and similar for desktop variant)
- Sections include `[Keyboard]` for shortcut customisation
- `-i "path/to/your.ini"` CLI flag to use alternate config
- Tools > Customize menu in app provides GUI for binding edit
- Multiple INI profiles common via import/export

### Performance characteristics (subjective community reports)
- Pre-decoding next pages
- Multi-threaded decode (claimed but unverified)
- Known weakness: **large ZIPs crash** (memory mismanagement per leopck README)
- Known weakness: single-threaded archive read can be a bottleneck

## Sources

- [Non-official atwiki (jp)](https://w.atwiki.jp/mangameeya) — comprehensive but 403 to automated fetchers
- [5ch software thread #24 (2025)](https://egg.5ch.net/test/read.cgi/software/1742548602/3-n) — ongoing user community
- [5ch software thread #18 (2018)](https://egg.5ch.net/test/read.cgi/software/1515748480/67-n)
- [マンガミーヤ プラグイン解説 (jp blog)](http://ykarenpgtm.dip.jp/link120.html)
- [Vector — RAR extract plug-in 詳細](https://www.vector.co.jp/soft/win95/art/se261123.html)
- [Goo Q&A — MangaMeeya 機能拡張](https://oshiete.goo.ne.jp/qa/3885889.html)
- [leopck/MangaMeeya (parallel Rust reimplementation)](https://github.com/leopck/MangaMeeya)

## Unknowns / TODO

- [ ] Full default keymap list (atwiki block needs different fetch path)
- [ ] Exact INI section names and key formats
- [ ] Whether original MangaMeeya supports Unicode filenames natively or relies on system code page
- [ ] Any documented "secret" features in the customise dialog

## Relation to this project

- We do **not** aim for bit-perfect compat. We do aim for **subjective speed parity** and **keymap-shape parity** (default bindings feel familiar).
- Susie shim in Phase 4 is the migration carrot — anyone with .spi collection should not feel they're abandoning it.
- 32-bit Susie plug-ins on our 64-bit binary is a known headache; Phase 4 will need to evaluate either (a) ship a 32-bit susie-host subprocess that pipes data, or (b) only support 64-bit SF-spec plug-ins. Decision deferred.
