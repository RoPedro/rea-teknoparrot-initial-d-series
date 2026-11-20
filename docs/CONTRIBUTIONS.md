# Contributions back

Drafts of what this launcher's own work found, for the places players and
upstream projects look (`sdk/docs/playbooks/contribute-back.md`). The
owner reviews and posts each one; nothing here has been posted unless its
status says so.

| Id | Target | What | Evidence | Status |
|---|---|---|---|---|
| C1 | [aderumier/windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader): an issue with the patch (or a PR) | `readonly_sections` breaks Initial D 8 on Windows itself: the dump's import thunks are in `.rdata` and it declares no IAT directory | `docs/RUNTIME_NOTES.md`, "Issue #1, still crashing on 0.4.1"; `patches/wal/0004` | draft, for the owner. Verified on a Windows 11 PC with the header changed by hand; a build with the patch not yet run there. Not in upstream as of 2026-11-20 (`main` at `9c927a0`) |

## C1: windowsarcadeloader, `readonly_sections` and Windows' loader

Title: `readonly_sections`: Initial D 8 crashes in ntdll on Windows (dump without an IAT directory)

> On Windows itself (not Wine), Initial D Arcade Stage 8 Infinity
> (`InitialD8_GLW_RE_SBZZ_redumped_.exe`, sha256
> `ce8e3c35cf41b08c922e42400187ddd525d27c2f1aae6d634659f918c63d7fec`)
> exits with `0xc0000005` in `ntdll.dll` about 3 s after start, before its
> entry point: the payload is never loaded and no `wal-ringedge.log` is
> written. Seen on Windows 11 (build 26300), the same `ntdll+0x4db0c`
> every time.
>
> The dump's import descriptors were rebuilt in `.mackt`, but its 23 thunk
> arrays are still at the start of `.rdata` (`0xb29000`–`0xb29e28`), and
> the IAT data directory (entry 12) is empty. The profile makes `.rdata`
> read-only (`readonly_sections`, needed against R6002). Windows' loader
> makes only the IAT directory's pages writable while it fills the thunks,
> so it writes into read-only memory. Wine's loader unprotects each thunk
> array itself, so Linux never shows it.
>
> Checked on that PC: the read-only exe started directly (no loader)
> crashes at the same address; with the IAT directory set to `0xb29000`,
> `0xe28` bytes, it loads, and `arcade-launcher run` reaches the game's
> startup checks with the payload connected.
>
> The patch below makes `readonly_sections` fill the IAT directory, when the
> image declares none, with the span of the thunk arrays that lie in the
> sections it makes read-only (null entries included); an image that has
> one, or whose imports are elsewhere, is unchanged. Three tests with
> synthetic PE32 images. On the real dump its output is identical to the
> exe that started. ID6, ID7 and Transformers use `readonly_sections` too;
> their dumps were not checked.
>
> (attach `patches/wal/0004-windows-iat-for-readonly-sections.patch`,
> made on top of this repository's other patches; the hunk applies to
> `crates/launcher/src/rundir.rs`)
