## Sandboxing Scenarios

- No file or directory has been provided in the policy
- Only directory permissions have been provided in the policy
- Both directory and file permissions are provided 
    - 



# Scenario 1

DirPerms::READ (without MUTATE) means the directory is read-only as a directory.

- Concretely, allowed:
    - listing the directory's contents
    - opening paths inside it — files and subdirectories
    - stat'ing entries, reading symlink targets

- Denied — every operation that changes the directory's structure:
    - creating a new file (an open with the CREATE flag)
    - deleting or renaming entries
    - creating subdirectories, symlinks, or hard links



READ + READ — the read-only mount. List the tree, open and read anything in it, change nothing. This is what you want for handing a guest input data, and it's the combination the wasmtime docs use in their own ./readonly example.

MUTATE + WRITE — the drop box. The guest can create files and write into them but cannot enumerate the directory or read anything back, including its own earlier output. Good for a log or artifact sink when you don't want the guest inspecting what else landed there. This is the least-travelled corner of the matrix, though, so verify it behaves as you expect on your wasmtime version rather than assuming.

READ + all — a fixed set of mutable files. The file set is frozen: no creating, deleting, or renaming. But every existing file's contents can be rewritten freely. Useful when you're handing over a known set of slots and want the shape of the directory to stay put.

READ + WRITE, and all + (empty) are the incoherent corners. The first lets a guest overwrite files it can't read; the second lets it delete and rename files whose contents it can't touch. If you've landed on either, you probably meant something else.

all + READ is the combination to avoid — and not only because it's the shape of the recent advisory. It's internally inconsistent as a security policy: MUTATE already lets the guest delete or rename any file, so withholding FilePerms::WRITE buys you no integrity protection. An attacker who wants to replace a file's contents just unlinks it. If you care about integrity, drop MUTATE.