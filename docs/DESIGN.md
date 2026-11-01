# phpz Design Document

## Goal

phpz is a simple PHP version manager inspired by `nvm`. It focuses on the **PHP CLI only**, without interfering with system-wide installations (Apache, Nginx, IIS, etc).

Beyond installing and switching versions, it aims to make `php.ini` management simple and safe.

---

## Status

| Area | Linux | macOS | Windows |
|---|---|---|---|
| install, uninstall, list --remote, use, list, current, env, reset, setup, completions | done | untested | done |
| config get, set, remove | done | untested | done |
| ext enable, disable | exits with an explanatory error | same | same |

---

## Directory Structure

```
~/.phpz/
  versions/
    8.3.32/
      bin/php        wrapper script
      libexec/php    real static binary
      php.ini
      conf.d/        additional ini files
    8.3.32.tmp/      exists only while an install is in progress
  current -> versions/8.3.32
```

A version is installed into `<version>.tmp` and renamed to `<version>` only when everything succeeded, so an interrupted install never leaves a half-installed version behind. A failed or leftover `.tmp` directory is ignored by `list` and removed on the next attempt.

On Windows the official zip is extracted as it comes, and `current` is a directory junction:

```
%USERPROFILE%\.phpz\
  versions\
    8.4.27\
      php.exe, DLLs, ext\     from the zip
      php.ini                 generated from php.ini-development
  current  ->  versions\8.4.27
```

A junction was chosen over a symlink because creating symlinks on Windows needs administrator rights or Developer Mode, while junctions do not. It was also chosen over a shim directory because `current` can be put on `PATH` as it is: the official zip keeps `php.exe` at its top level, so no `bin` directory or wrapper is needed.

`PHPZ_HOME`, when set, replaces `~/.phpz` (or `%USERPROFILE%\.phpz`). It exists mostly so tests can run in an isolated directory, since `dirs::home_dir()` on Windows ignores environment variables.

---

## Where PHP binaries come from

Linux and macOS have no official prebuilt PHP binaries. The options considered were:

| Option | Verdict |
|---|---|
| Compile from source | Rejected for now: slow (minutes per version), needs a compiler and many `-dev` libraries, fails differently on every distribution. |
| **Prebuilt static binaries (static-php-cli)** | **Chosen**: installs in seconds, no dependencies, same behavior on every distribution. |
| Distribution packages (apt, dnf) | Rejected: needs `sudo`, is distribution specific and installs system-wide, which is what phpz avoids. |

Binaries are downloaded from `https://dl.static-php.dev/static-php-cli/common/`, using the file name pattern `php-<version>-cli-<os>-<arch>.tar.gz`. The directory index is available as JSON (`?format=json`), and phpz uses it as the source of truth for which versions exist, so `phpz install 8` resolves to the newest 8.x.y that actually has a build.

Each archive contains a single `php` executable. phpz extracts it to `libexec/php`.

### The wrapper script

The static binary has its `php.ini` search path compiled in (`/usr/local/etc/php`). `bin/php` is a small shell script that sets `PHPRC` and `PHP_INI_SCAN_DIR` to the version's own directory and then runs `libexec/php`. This is what gives every version an independent configuration.

### Windows

Windows has official builds, so they are used. `https://windows.php.net/downloads/releases/releases.json` lists the newest release of each series (7.4 to 8.5 today) with the file name and the **SHA-256** of every zip. phpz picks the non-thread-safe x64 build (`nts-*-x64`, the right one for the CLI), downloads it and compares the SHA-256 before anything is extracted. A mismatch discards the download.

- Versions can carry a packaging revision (`8.3.36-1`). It is dropped from the version phpz shows and stores (`8.3.36`) but kept in the download URL, because the zip is named `php-8.3.36-1-nts-Win32-vs16-x64.zip`.
- Zip entries that try to leave the destination (`../`) are refused.
- The zip contains no `php.ini`. phpz writes one from the bundled `php.ini-development`, activating `extension_dir = "ext"` and a default set of extensions (bz2, curl, exif, fileinfo, gd, gmp, intl, mbstring, mysqli, openssl, pdo_mysql, pdo_pgsql, pdo_sqlite, pgsql, sockets, sodium, sqlite3, zip). Directives are matched even when the template puts a comment after them (`;extension=exif ; Must be after mbstring...`).
- Older patches live in the `archives` directory, which has no JSON index, so phpz reads its HTML listing (about 740 KB). File names are parsed strictly: only `php-<version>[-<revision>]-nts-Win32-<toolchain>-x64.zip` counts, so thread-safe and x86 builds, devel/debug/test packs and SBOM files are ignored. When a version has several rebuilds (`8.5.12` and `8.5.12-1`) the highest revision wins, and an entry from `releases.json` wins over the archive entry of the same version.
- The floor is PHP 7.0, the oldest series that runs with the current Visual C++ redistributable (VC14). 5.x needs VC11, which a modern machine usually lacks, so it is not offered. The archive has a single zip for each of 7.0, 7.1, 7.2 and 7.3 (the final release of the series). Their `php.ini-development` differs from newer ones: 7.0 and 7.1 name extensions by DLL (`;extension=php_curl.dll`), and 7.0 to 7.2 write `; extension_dir = "ext"` with a space after the semicolon. The generator picks DLL names below 7.2 and accepts the spaced form for `extension_dir` only, so a prose comment such as `; extension=curl` is never activated by accident.
- The page is read only when needed. A request for a series or a minor (`8`, `8.3`) always resolves to its newest release, which `releases.json` already holds, so `phpz install 8.3` and `phpz list --remote` stay at one small request. Only an exact older patch (`8.3.12`) and `phpz list --remote --all` read the archive page.
- **Checksums for archive builds.** The archive publishes no usable hash file (`sha1sum.txt` only covers PHP 5.2 era files). Builds from 8.2.33, 8.3.33, 8.4.24 and 8.5.9 onwards have a CycloneDX SBOM next to the zip (`<zip>.cdx.json`) whose `metadata.component.hashes` is the SHA-256 of the zip itself (checked against the real file). phpz fetches it when installing and verifies the download. A missing SBOM (HTTP 404) means no checksum is published. Any other failure to fetch it aborts the install, so a flaky connection can never silently skip verification.
- **Consent.** A build carries `expects_checksum`, true when its source normally publishes one (Windows) and false otherwise (static-php publishes none at all). If such a build has no checksum, the install needs the user's consent: a person at a terminal is asked (default no), a script is refused with a message, and `--allow-unverified` gives consent explicitly. The rule is a pure function of three booleans, so it is unit tested. Linux and macOS are exempt for now, because every build there is unverified and asking each time would only be noise; they keep the one-line note until a more official source replaces static-php.

### Trade-offs

- **Fixed extension set.** The build includes: bcmath, bz2, calendar, ctype, curl, dom, exif, fileinfo, filter, ftp, gd, gmp, iconv, libxml, mbstring, mbregex, mysqlnd, openssl, pcntl, pdo, pdo_mysql, pdo_pgsql, pdo_sqlite, pgsql, phar, posix, redis, session, simplexml, soap, sockets, sqlite3, tokenizer, xmlreader, xmlwriter, zip and zlib. A static binary cannot load external `.so` extensions, so `xdebug`, `intl` or `opcache` are not available.
- **Third-party dependency.** If the upstream host changes its layout or disappears, installs break.
- **No integrity check (Linux and macOS).** Upstream publishes no checksums, so downloads are only checked for completeness (received bytes match `Content-Length`), and phpz prints a note saying so. Pinning hashes is a planned improvement. Windows downloads are verified.

---

## Version Management

- `phpz install <version>`: resolve the version, download, extract and lay out `versions/<version>/`. Accepts `MAJOR`, `MAJOR.MINOR` or `MAJOR.MINOR.PATCH`.
- `phpz uninstall <version>`: remove an installed version. It requires the exact version shown by `list` (so `8.3` never deletes by accident) and refuses to remove the active version.
- `phpz list --remote [--all]`: list versions that have a build for this platform, newest patch of each minor by default, marking the ones already installed. `--all` is only accepted together with `--remote`.
- `phpz use <version>`: point `current` at an installed version. On Unix the symlink is replaced atomically (create a temporary link, then rename over `current`). On Windows a junction cannot be renamed over, so the old one is removed and a new one created. Partial versions match the newest installed one.
- `phpz list`: list installed versions and mark the active one.
- `phpz current`: show the active version.
- `phpz env`: print shell code that puts the active version first on `PATH`: an `export` line for POSIX shells, and `$env:Path = "...;" + $env:Path` for PowerShell (use `phpz env | Invoke-Expression`).
- `phpz reset`: remove the `current` link (symlink or junction, never the version it points to), deactivating phpz's PHP.

### Environment strategy

A child process cannot change its parent shell's environment, so phpz cannot edit `PATH` by itself. A one-time change to the shell profile is unavoidable, and phpz does it for the user:

- `phpz setup` appends a marked block to `~/.bashrc` or `~/.zshrc` (`$ZDOTDIR` is honored):

  ```bash
  # >>> phpz >>>
  export PATH="$HOME/.phpz/current/bin:$PATH"
  # <<< phpz <<<
  ```

  `phpz setup --undo` removes exactly that block (and the blank line `setup` put before it), so a `setup` followed by `--undo` restores the file byte for byte. If the start marker has no matching end marker, it refuses to edit the file. It is idempotent (the start marker is checked first) and writes the file in place, so symlinked dotfiles keep working. The block exports the path directly instead of running `eval "$(phpz env)"`, which avoids starting a process on every new shell and cannot fail when `phpz` is missing from `PATH`.
- After a successful `phpz install`, phpz offers to run the same setup. The question is asked only once (recorded in `~/.phpz/.setup-prompted`), only when stdin and stderr are terminals, and not when `PATH` is already configured. Scripts and CI are never prompted.
- `phpz env` still prints the export line for shells that `setup` does not support.

`PATH` points to the stable `~/.phpz/current/bin`, so `phpz use` takes effect immediately in every open shell without re-evaluating anything. On Unix only bash and zsh are supported by `setup`.

**Windows.** There is no profile file to edit; the `PATH` lives in the registry, so `phpz setup` edits the **user** `PATH` (`HKEY_CURRENT_USER\Environment`, value `Path`):

- It puts `<phpz home>\current` **first**, so phpz wins over other user entries such as Scoop's shims, and it matches existing entries ignoring case and a trailing backslash, which makes it idempotent.
- It writes the value back with the **same registry type** (`REG_EXPAND_SZ` normally), so entries such as `%USERPROFILE%\bin` keep expanding. Every other entry stays byte for byte as it was, and `setup --undo` restores the original value exactly.
- It then broadcasts `WM_SETTINGCHANGE` so Explorer and newly opened terminals see the change. Terminals that are already open keep their old `PATH`; the message printed by `setup` shows the one-liner that updates the current one.
- Windows builds a program's `PATH` as the system `PATH` followed by the user `PATH`. A PHP on the *system* `PATH` is therefore found first, and phpz does not touch the system `PATH` because that needs administrator rights.

### Isolation between versions

On Unix, `bin/php` sets `PHPRC` and `PHP_INI_SCAN_DIR` itself. On Windows `php.exe` runs directly and inherits the environment, so a `PHP_INI_SCAN_DIR` left by another PHP installation (Scoop sets one) makes PHP read that installation's ini files as well, and every extension is reported as "already loaded". `phpz use` warns when either variable is set, offers `$env:NAME = $null` to clear it for the current terminal first, and mentions the permanent removal last because it also affects the other PHP. A small `php.exe` shim that clears the variables before starting the real one would remove the need for the warning; it is not implemented.

`reset` does not clean other PHP installations out of `PATH`. It only deactivates phpz's own version.

---

## php.ini Management

Each version has its own `php.ini` under `versions/<version>/`. A default is written at install time and users are free to edit it.

### Config commands

```bash
phpz config get memory_limit
phpz config set memory_limit 512M
phpz config remove memory_limit
```

They operate on the `[PHP]` section of the active version's `php.ini`, on every platform.

### The ini editor

`src/ini.rs` is a **line-preserving editor**, not a parse-and-regenerate library. The file is kept as a list of lines and only the lines that match the requested key are changed, so:

- comments, blank lines and ordering are preserved;
- repeated keys (`extension=curl`, `extension=gd`) are preserved;
- CRLF line endings are preserved;
- the last definition of a key wins, as in PHP;
- a new key is inserted after the last non-blank line of its section, and the section is created if it does not exist;
- the file is saved through a temporary file and a rename.

An earlier draft stored the file in hash maps. That approach was dropped because it loses comments, ordering and repeated keys on every write.

Values are written exactly as the user typed them. phpz never adds quotes, because in `php.ini` a quoted value is a string while an unquoted one may be a constant expression such as `E_ALL & ~E_NOTICE`.

### Extensions (planned)

`phpz ext enable|disable <name>` is not implemented. With the current static binaries it would have no effect, since extensions are compiled in and external ones cannot be loaded. It becomes meaningful when phpz supports builds that load `.so` files, for example an optional compile-from-source install. Until then the command exits with code 1 and an explanatory message.

### Profiles (planned)

Keeping several `php.ini` profiles per version, such as `dev.ini` (verbose errors, Xdebug) and `prod.ini` (optimized, opcache), selected with `phpz config use <profile>`.

---

## Code Layout

```
src/
  main.rs            entry point
  paths.rs           phpz home (PHPZ_HOME), versions directory, current link, directory for PATH
  version.rs         builds, version selection
  version/           static_php.rs (Linux, macOS) and windows_php.rs (windows.php.net)
  installed.rs       installed versions, active version, active php.ini path
  link.rs            symlink (Unix) or junction (Windows) for `current`
  shell.rs           SetupTarget: shell profile block (Unix) or user PATH (Windows), first-install question
  user_path.rs       text edits of the Windows PATH and its registry access
  environment.rs     warning about PHP ini variables that break isolation
  interaction.rs     interactive terminal check and the yes/no question
  pipe.rs            quiet exit when the reader of our output goes away
  ini.rs             line-preserving php.ini editor
  commands/          one module per subcommand
  <module>/tests.rs  unit tests of <module>.rs (private items stay reachable)
tests/fixtures/      sample files used by the tests
```

Version and installed-version selection are pure functions (`select_build`, `select_installed`), and so are the parsers and text edits that Windows needs (`parse_releases`, `add_entry`, `remove_entry`, `uncomment_directives`), so all of them are unit tested on every platform without network or registry access. `tests/closed_pipe.rs` runs the real binary with a closed output pipe. CI (`.github/workflows/ci.yml`) runs build, tests and `clippy -D warnings` on Linux and Windows.

Closing the output pipe early (`phpz list --remote | head -1`) used to end in a panic message. On Unix the default `SIGPIPE` behavior is restored at startup, and a panic hook turns a failed write caused by a broken pipe into a quiet exit, which is what covers Windows.

`phpz completions <shell>` prints a completion script generated from the command definitions.

---

## Cross-platform Support

- **Linux**: static tar.gz builds and symlinks. Supported.
- **macOS**: the same upstream provides builds for Intel and ARM64, and the code path is shared with Linux, but it has not been tested.
- **Windows**: official zip builds from windows.php.net (x64, non-thread-safe), verified with SHA-256, a junction for `current`, and the user `PATH` in the registry. Supported and tested on Windows 11. ARM64 is not supported because there are no official ARM64 builds.

---

## Future Enhancements

- A `php.exe` shim that clears the ini variables, replacing the Windows warning.
- Download integrity verification with pinned checksums on Linux and macOS, and for the older Windows patches that publish none.
- Optional compile-from-source install for custom extensions, which would also enable `phpz ext`.
- `php.ini` profiles and templates (dev, prod, testing).
- Global configuration shared across versions.
- Integration with Docker images for reproducibility.
