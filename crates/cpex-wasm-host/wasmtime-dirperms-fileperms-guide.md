# WASI Preopen Permissions: `DirPerms` × `FilePerms`

A scenario-by-scenario reference for `wasmtime_wasi` filesystem preopens.

---

## The mental model

Two orthogonal flag sets, each with two bits:

| Flag set | Governs | Flags |
|---|---|---|
| `DirPerms` | The **namespace** — which names exist in the directory, and whether you can add or remove them | `READ`, `MUTATE` |
| `FilePerms` | The **bytes** — what you can do with a file's contents once it's open | `READ`, `WRITE` |

Almost every confusing combination comes from conflating these. "Can I open `x.txt`?" is a `DirPerms` question. "Can I write into the handle I got back?" is a `FilePerms` question.

**What each `DirPerms` flag grants:**

- `READ` — iterate the directory's entries, open paths inside it (files *and* subdirectories), stat entries, read symlink targets.
- `MUTATE` — create new files, delete and rename entries, create subdirectories, symlinks, and hard links.

**What each `FilePerms` flag grants:**

- `READ` — read the contents of an open file.
- `WRITE` — write to the contents of an open file.

Denied operations fail with `error-code.not-permitted` (wasip2) or `ERRNO_PERM` (wasip1).

### Three rules that apply everywhere

1. **The preopen is the security boundary.** Permissions narrow what can be done *inside* a preopen; they cannot carve a hole out of the middle of one. There is no per-file allowlist. If a file must not be reachable, it must not be inside the preopened directory.
2. **Permissions are recursive.** Subdirectories opened through a preopen inherit the same `DirPerms` and `FilePerms`. The flags apply to the whole tree, not just the top level.
3. **Empty is inert, not read-only.** `DirPerms::empty()` denies opening as well as listing, so the guest receives a handle it can do nothing whatsoever with.

---

## The grid at a glance

Rows are `DirPerms`, columns are `FilePerms`. ✅ marks a coherent policy, ⚠️ a combination that usually indicates a mistake.

| | *(empty)* | `READ` | `WRITE` | `all` |
|---|---|---|---|---|
| ***(empty)*** | inert | inert | inert | inert |
| **`READ`** | list names only | ✅ read-only mount | ⚠️ blind overwrite | ✅ fixed set of mutable files |
| **`MUTATE`** | create empty files | ⚠️ contradictory | ✅ write-only drop box | private scratch, no listing |
| **`all`** | ⚠️ restructure, no contents | ⚠️ **avoid** | full control, blind writes | ✅ full access |

---

## Scenario 1 — Read-only mount ✅

**`DirPerms::READ` + `FilePerms::READ`**

```rust
wasi.preopened_dir("./corpus", "/data", DirPerms::READ, FilePerms::READ);
```

| Allowed | Denied |
|---|---|
| List entries | Create files |
| Open files and subdirectories | Delete or rename |
| Read file contents | Write to any file |
| Stat entries, read symlinks | Create dirs, symlinks, hard links |

**Use it for:** handing a guest input data — a corpus to analyze, config files, a template set, static assets to be served. This is the workhorse combination and the one the wasmtime docs use for their own `./readonly` example.

**Watch out:** the guest can enumerate every filename in the tree. If a *name* is sensitive, this combination still leaks it.

---

## Scenario 2 — Full access ✅

**`DirPerms::all()` + `FilePerms::all()`**

```rust
wasi.preopened_dir("./workspace", "/", DirPerms::all(), FilePerms::all());
```

Everything is permitted within the preopen. The sandbox boundary still holds — `..` cannot traverse above `host_path`, and symlinks resolving outside the root are rejected — but inside, the guest has free rein.

**Use it for:** a scratch workspace, a build directory, a genuinely trusted guest. The `wasmtime` CLI uses `FilePerms::all()` for all its preopens.

---

## Scenario 3 — Write-only drop box ✅

**`DirPerms::MUTATE` + `FilePerms::WRITE`**

```rust
wasi.preopened_dir("./artifacts", "/out", DirPerms::MUTATE, FilePerms::WRITE);
```

| Allowed | Denied |
|---|---|
| Create files | List the directory |
| Write to files it creates | Read anything back |
| Delete and rename | Open a file for reading |

The guest can deposit output but cannot enumerate the directory or read anything — not even its own earlier writes.

**Use it for:** a log sink or artifact output directory where you don't want the guest inspecting what other tenants have deposited alongside it.

**Watch out:** this is the least-travelled corner of the grid. Verify the behavior on your specific wasmtime version rather than assuming — and note that `MUTATE` alone still permits deletion, so a guest can destroy the drop box's contents even though it can't read them.

---

## Scenario 4 — Fixed set of mutable files ✅

**`DirPerms::READ` + `FilePerms::all()`**

```rust
wasi.preopened_dir("./slots", "/state", DirPerms::READ, FilePerms::all());
```

The *set* of files is frozen — nothing can be created, deleted, or renamed — but the contents of every existing file can be read and rewritten freely.

**Use it for:** handing over a known set of slots (a fixed group of state or scratch files) where the shape of the directory must stay put. Also a reasonable shape for a database or journal file the guest updates in place.

---

## Scenario 5 — List names only

**`DirPerms::READ` + `FilePerms::empty()`**

The guest can enumerate entries and stat them, but every attempt to read or write contents fails.

**Use it for:** a narrow case — letting a guest discover *what* is available without seeing any of it, e.g. so it can request specific items back through a host-provided API instead. Rarely what you actually want.

---

## Scenario 6 — Create empty files

**`DirPerms::MUTATE` + `FilePerms::empty()`**

The guest can create, delete, and rename entries, but cannot put bytes into anything or read anything.

**Use it for:** essentially nothing real. It shows up as a lock-file or marker-file mechanism, where the file's *existence* carries the signal and its contents are irrelevant.

---

## Scenario 7 — Private scratch, no listing

**`DirPerms::MUTATE` + `FilePerms::all()`**

Full read/write on file contents and full ability to restructure — but no directory enumeration. The guest can use any path it already knows or constructs, it just can't discover what's there.

**Use it for:** a scratch space where you'd rather the guest not survey its surroundings. The lack of listing is a mild obstacle, not a real confidentiality barrier: a guest that can guess or derive filenames can still open them.

---

## Scenario 8 — Blind overwrite ⚠️

**`DirPerms::READ` + `FilePerms::WRITE`**

The guest can open existing files and overwrite them, but never read them back and never create new ones.

This is almost always a mistake. If you meant "the guest writes output here," it needs `MUTATE` to create files. If you meant "the guest updates existing files," it probably needs `FilePerms::all()`.

---

## Scenario 9 — Restructure without contents ⚠️

**`DirPerms::all()` + `FilePerms::empty()`**

The guest can delete, rename, and create entries, but cannot read or write any file's bytes. It can destroy your data wholesale while being unable to read a single byte of it. Rarely a coherent policy.

---

## Scenario 10 — Contradictory ⚠️

**`DirPerms::MUTATE` + `FilePerms::READ`**

The guest can create and delete files, and can read contents — but cannot write. Since it can create files it then cannot fill, and delete files it can read, the policy doesn't express a sensible intent. See the warning below.

---

## Scenario 11 — `DirPerms::all()` + `FilePerms::READ` ⚠️ Avoid

This combination deserves its own note, on two grounds.

**It is incoherent as a security policy.** `MUTATE` already permits deleting and renaming any file. Withholding `FilePerms::WRITE` therefore buys no integrity protection at all — an attacker who wants to replace a file's contents simply unlinks it and creates a new one under the same name. If integrity is the goal, the flag to drop is `MUTATE`, not `WRITE`.

**It was also the shape of a real vulnerability.** A 2026 advisory (CVE-2026-47261, CVSS 7.5) covered exactly this configuration: with `MUTATE` on the directory and `READ`-only on files, a `path_open` / `open-at` call using only the `TRUNCATE` flag bypassed the `FilePerms::WRITE` check, because the truncate path failed to set write mode before the access-control test. Fixed in wasmtime-wasi 24.0.9, 36.0.10, and 44.0.2. Only embeddings combining `DirPerms::MUTATE` with `FilePerms::READ` were affected; the wasmtime CLI was not, since it always passes `FilePerms::all()`.

---

## Scenario 12 — Inert (any `FilePerms`)

**`DirPerms::empty()`**

Because opening a path requires `DirPerms::READ`, an empty `DirPerms` makes the preopen useless regardless of what `FilePerms` says. The guest holds a directory handle it cannot list, cannot open through, and cannot reach any file with. `FilePerms` is unreachable and therefore irrelevant.

If you want the guest to have no filesystem access, don't configure the preopen at all — that's the default state.

---

## Choosing quickly

| I want the guest to… | `DirPerms` | `FilePerms` |
|---|---|---|
| Read some input data and nothing else | `READ` | `READ` |
| Produce output files it can also read back | `all()` | `all()` |
| Deposit output it cannot read back | `MUTATE` | `WRITE` |
| Update a fixed set of files, no new ones | `READ` | `all()` |
| Do anything at all in a scratch area | `all()` | `all()` |
| Have no filesystem access | *don't preopen* | — |

---

## Related: per-file access

If you need "read `input.txt` but not `secrets.txt` in the same directory," no flag combination expresses it — permissions are per-preopen. Options:

1. **Restructure the host layout.** Build a staging directory containing only the shared file (a hard link avoids copying bytes and keeps contents live) and preopen that. Note that a symlink pointing back out to the original will be rejected by the sandbox. On Linux, a read-only bind mount is an alternative.
2. **Skip the filesystem.** If the guest reads one file start to finish, pipe it in via `WasiCtxBuilder::stdin` and configure no preopen at all.
3. **Virtualize.** WASI-Virt can deny host preopens and construct a virtual filesystem with an explicit entry list, composed into the component — giving you the per-file allowlist the flags don't provide.

---

## Sources

- [`DirPerms` API docs](https://docs.wasmtime.dev/api/wasmtime_wasi/filesystem/struct.DirPerms.html)
- [`WasiCtxBuilder::preopened_dir`](https://docs.wasmtime.dev/api/wasmtime_wasi/struct.WasiCtxBuilder.html)
- [GHSA-2r75-cxrj-cmph — path_open(TRUNCATE) bypasses `FilePerms::WRITE`](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-2r75-cxrj-cmph)
- [WASI-Virt](https://github.com/bytecodealliance/WASI-Virt)
