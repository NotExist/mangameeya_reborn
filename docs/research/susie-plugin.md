# Susie Plug-in — Research Notes

## Status

Susie plug-in is a Japanese de-facto standard for image viewer plug-ins, originally for the **Susie** image viewer. The spec is **actively maintained as of 2026-02-11** at [TORO's Library](https://toro.d.dooo.jp/dlsphapi.html), now covering both 32-bit and 64-bit variants.

## Plug-in types (via `GetPluginInfo` infono=0)

| Tag | Meaning |
|---|---|
| `00IN` | Image expansion plug-in — converts image bytes to BITMAP |
| `00AM` | Archive expansion plug-in — list / extract entries from archive |
| `T0XN` (SF) | 64-bit variants of the above |

## Call flow

### Image plug-in
```
IsSupported(filename, bytes)
  → GetPictureInfo(filename, bytes) [optional]
  → GetPicture(filename, bytes, callback) → BITMAP
```

### Archive plug-in
```
IsSupported(filename, bytes)
  → GetArchiveInfo(filename, bytes) → fileInfo[]
  → for each entry:
      GetFile(filename, bytes, output_buffer, callback) → extracted bytes
      (or GetFileInfo for metadata-only)
```

## Memory ownership

- Caller passes input bytes
- Plug-in allocates output (image / file data)
- Caller frees via `LocalFree` (Win32) — historical Windows convention
- Progress callback returns 0 to continue, non-zero to abort

## Key data structures (abridged)

```c
typedef struct PictureInfo {
    long left, top;
    long width, height;
    WORD x_density, y_density;
    short colorDepth;
    HLOCAL hInfo;            // optional EXIF / metadata blob
} PictureInfo;

typedef struct fileInfo {
    char method[8];          // compression method
    DWORD position;          // offset in archive
    DWORD compsize;
    DWORD filesize;
    DWORD timestamp;
    char path[200];
    char filename[200];
    DWORD crc;
} fileInfo;
```

## Limitations for our purposes

1. **Windows-only by design.** API is Win32 (HLOCAL, DWORD, BITMAP). No portable equivalent.
2. **32-bit history.** Many ecosystem .spi are 32-bit DLLs; running them from a 64-bit process needs subprocess bridging.
3. **Filename encoding ambiguity.** Older spec uses `char[]` assuming Shift-JIS / system code page. The SF spec (64-bit) introduces wide-char variants but support is uneven.
4. **No sandbox.** A malicious .spi has full process access.
5. **Single-threaded assumption.** Many .spi are not thread-safe.

## Why we are not adopting Susie as our primary plug-in API

- Cross-platform impossible (point 1)
- Encoding pitfalls with CJK content (point 3) — exactly our target audience
- No sandbox (point 4) — modern users expect safety
- Cannot define a Rust-idiomatic API on top of Win32 conventions cleanly

## Why we still support Susie as a Windows shim (Phase 4)

- Migration path for users with existing .spi collections
- Some niche formats only have .spi implementations (legacy galge image formats, certain archive flavours)
- Effort is bounded: a single `mm-susie-shim` crate behind a Windows-only feature flag

## Implementation sketch (Phase 4)

```rust
// crates/mm-susie-shim/src/lib.rs
#[cfg(target_os = "windows")]
pub struct SusiePlugin {
    handle: HMODULE,
    kind: PluginKind,
    funcs: SusieVtable,
}

#[cfg(target_os = "windows")]
impl ArchivePlugin for SusiePlugin {
    fn open(&self, bytes: &[u8]) -> Result<Vec<EntryInfo>> {
        // call GetArchiveInfo via vtable, translate fileInfo[]
    }
    // ...
}
```

For 32-bit .spi compatibility from a 64-bit host: we will either ship a small 32-bit `mm-susie-host.exe` subprocess that loads .spi and pipes results over stdio, or restrict to 64-bit SF-spec plug-ins. Decision in Phase 4.

## Sources

- [TORO's Library — Susie 32bit/64bit Plug-in 仕様 (2026-02-11)](https://toro.d.dooo.jp/dlsphapi.html)
- [Susie Plug-in Specification Rev.4 (GetPluginInfo)](https://www2f.biglobe.ne.jp/~kana/spi_api/spi_getplugininfo.html)
- [Susie プラグイン解体新書 (annotated impl notes)](http://www.asahi-net.or.jp/~kh4s-smz/spi/note/spiapimp.html)
- [How to create a Susie Plug-in (tutorial)](http://www2f.biglobe.ne.jp/~kana/howtospi.html)
- [Susie Plug-in 関係の工作室 (resources hub)](https://www2f.biglobe.ne.jp/~kana/develop.html)
- [lioncash/ExtractData — Susie.cpp (open-source consumer reference)](https://github.com/lioncash/ExtractData/blob/master/Susie.cpp)
- [toroidj/runspx — Susie plug-in test runner](https://github.com/toroidj/runspx)
