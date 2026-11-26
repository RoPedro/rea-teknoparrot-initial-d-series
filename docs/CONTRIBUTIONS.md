# Contributions back

Drafts of what this launcher's own work found, for the places players and
upstream projects look (`sdk/docs/playbooks/contribute-back.md`). The
owner reviews and posts each one; nothing here has been posted unless its
status says so.

| Id | Target | What | Evidence | Status |
|---|---|---|---|---|
| C1 | [aderumier/windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader): an issue with the patch (or a PR) | `readonly_sections` breaks Initial D 8 on Windows itself: the dump's import thunks are in `.rdata` and it declares no IAT directory | `docs/RUNTIME_NOTES.md`, "Issue #1, still crashing on 0.4.1"; `patches/wal/0004` | draft, for the owner. Verified on a Windows 11 PC with the header changed by hand; a build with the patch not yet run there. Not in upstream as of 2026-11-20 (`master` at `9c927a0`) |
| C2 | [aderumier/windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader): an issue with the patch (or a PR) | the RingEdge payload's hot patches go live before their original is stored: Initial D 8 (`WAL_RINGEDGE_ZERO_HEAP`) crashes at start now and then on Windows | `docs/RUNTIME_NOTES.md`, "warmup"; `patches/wal/0006` | draft, for the owner; **needs verification** (a build with the patch over many starts on Windows). Not in upstream as of 2026-11-21 (`master` at `9c927a0`, where `hotpatch` moved to `wal_payload_common::cg`: the patch needs adapting there) |
| C3 | [aderumier/windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader): an issue with the patch (or a PR) | Initial D 8 can't be closed with Alt+F4 on Windows: it only sets a flag on `WM_CLOSE`, and Alt+F4 never reaches its `WAL_WINDOW_POPUP` window; a `WAL_WINDOW_CLOSE` option ends the game on close requests | `docs/RUNTIME_NOTES.md`, "Initial D 8 ignoring Alt+F4"; `patches/wal/0007` | draft, for the owner. Verified on a Windows 11 PC with a CI build of the patch (Alt+F4 ended ID8, exit code 0, no leftovers). Not in upstream as of 2026-11-25 (`master` at `2bfe069`) |

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

## C2: windowsarcadeloader, a hot patch live before its original

Title: RingEdge payload: `RtlAllocateHeap` hook live before its original is stored (Initial D 8 crashes at start now and then on Windows)

> On Windows itself (not Wine), Initial D Arcade Stage 8 Infinity sometimes
> exits with `0xc0000005` right after start; the next start usually works.
> The payload's log of a failed start (Windows 11, build 26300), in the
> same millisecond as `ringedge: heap blocks zeroed`:
>
> ```
> crash: [tid 6824] access violation
> crash: access violation (reading 0x0) at 0x0
> crash: eip=00000000 eax=000c0008 ... (stack: heap handle, flags 0x000c0008, size 0x14, a return into wal_ringedge.dll)
> ```
>
> `zero_heap` (`WAL_RINGEDGE_ZERO_HEAP=1`, the ID8 profile) hot patches
> `ntdll!RtlAllocateHeap`: `hotpatch` writes the entry's short jump, which
> makes `allocate_heap` live for every thread, and returns; only then does
> `zero_heap` store the original in `ORIG_ALLOCATE_HEAP`. A thread
> allocating in between calls the original through a null pointer. Windows
> 10/11 run loader worker threads in each new process, and they allocate
> while the payload starts; Wine has none, so Linux never shows it. The
> `wglSwapBuffers` and `kernelbase` `CreateFileW` / `GetFileAttributesExW`
> hot patches store their original the same way.
>
> The original is known before patching (the function past its 2 byte
> prologue, `f + 2`). The patch below makes `hotpatch` take the static that
> holds it and store it, with a release fence, before the entry's 2 byte
> jump; the write order is in a small std-only module with two tests (the
> first fails with the old order). It is made against `4f42c6d`; on
> `master` `hotpatch` lives in `payload-common/src/cg.rs`, and the same
> change applies there.
>
> (attach `patches/wal/0006-ringedge-hotpatch-original-before-live.patch`)

## C3: windowsarcadeloader, close requests that end the game

Title: Initial D 8 ignores Alt+F4 / WM_CLOSE on Windows: a `WAL_WINDOW_CLOSE` option

> On Windows itself, Initial D Arcade Stage 8 Infinity
> (`InitialD8_GLW_RE_SBZZ_redumped_.exe`, sha256
> `ce8e3c35cf41b08c922e42400187ddd525d27c2f1aae6d634659f918c63d7fec`) can't be
> closed with Alt+F4: the player has to stop it from the launcher or the Task
> Manager.
>
> Two things. The game's window procedure (`+0x7996a0`) answers `WM_CLOSE`
> only by setting bit `0x2` of a flags word (`0x1438c18`) and returning 0;
> its message loop ends the game on other bits, never that one. And with the
> profile's `WAL_WINDOW_POPUP` the window has no `WS_SYSMENU`, so Alt+F4
> doesn't become a `WM_CLOSE` there at all. Read live on Windows 11 (the
> word sampled every 50 ms): Alt+F4 left bit `0x2` clear; a posted
> `WM_CLOSE` set it within 60 ms and the game kept running.
>
> The patch below adds `WAL_WINDOW_CLOSE=1` to `payload-common`'s
> `window.rs`: the payload puts a procedure in front of the game's top-level
> windows when they're created (the window's ANSI or Unicode kind kept), and
> Alt+F4 (`WM_SYSKEYDOWN` F4 with Alt held), `SC_CLOSE` or `WM_CLOSE` end the
> process, as stopping it from the launcher does. The message classification
> is in a std-only `close_request.rs` with tests. The ID8 profile turns it
> on; ID6 and ID7 probably share the window code but weren't checked.
>
> (attach `patches/wal/0007-window-close-request-ends-the-game.patch`, made
> against `4f42c6d` on top of this repository's other patches)
