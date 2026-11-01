# phpz Design Document

## Goal

phpz is a simple PHP version manager inspired by `nvm`. It focuses on the **PHP CLI only**, without interfering with system-wide installations (Apache, Nginx, IIS, etc).

Beyond installing and switching versions, it aims to make `php.ini` management simple and safe.

---

## Status

| Area | Linux | macOS | Windows |
|---|---|---|---|
| install, uninstall, list --remote, use, list, current, env, reset, setup, completions | done | untested | not implemented |
| config get, set, remove | done | untested | not implemented |
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

Windows is planned to use `%USERPROFILE%\.phpz\` with `php.exe` and a shim directory instead of a symlink.

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

### Trade-offs

- **Fixed extension set.** The build includes: bcmath, bz2, calendar, ctype, curl, dom, exif, fileinfo, filter, ftp, gd, gmp, iconv, libxml, mbstring, mbregex, mysqlnd, openssl, pcntl, pdo, pdo_mysql, pdo_pgsql, pdo_sqlite, pgsql, phar, posix, redis, session, simplexml, soap, sockets, sqlite3, tokenizer, xmlreader, xmlwriter, zip and zlib. A static binary cannot load external `.so` extensions, so `xdebug`, `intl` or `opcache` are not available.
- **Third-party dependency.** If the upstream host changes its layout or disappears, installs break.
- **No integrity check.** Upstream publishes no checksums, so downloads are only checked for completeness (received bytes match `Content-Length`). Pinning hashes is a planned improvement.

---

## Version Management

- `phpz install <version>`: resolve the version, download, extract and lay out `versions/<version>/`. Accepts `MAJOR`, `MAJOR.MINOR` or `MAJOR.MINOR.PATCH`.
- `phpz uninstall <version>`: remove an installed version. It requires the exact version shown by `list` (so `8.3` never deletes by accident) and refuses to remove the active version.
- `phpz list --remote [--all]`: list versions that have a build for this platform, newest patch of each minor by default, marking the ones already installed. `--all` is only accepted together with `--remote`.
- `phpz use <version>`: point the `current` symlink at an installed version. The symlink is replaced atomically (create a temporary link, then rename over `current`). Partial versions match the newest installed one.
- `phpz list`: list installed versions and mark the active one.
- `phpz current`: show the active version.
- `phpz env`: print shell code that puts `~/.phpz/current/bin` first on `PATH`.
- `phpz reset`: remove the `current` symlink, deactivating phpz's PHP.

### Environment strategy

A child process cannot change its parent shell's environment, so phpz cannot edit `PATH` by itself. A one-time change to the shell profile is unavoidable, and phpz does it for the user:

- `phpz setup` appends a marked block to `~/.bashrc` or `~/.zshrc` (`$ZDOTDIR` is honored):

  ```bash
  # >>> phpz >>>
  export PATH="$HOME/.phpz/current/bin:$PATH"
  # <<< phpz <<<
  ```

  It is idempotent (the start marker is checked first) and writes the file in place, so symlinked dotfiles keep working. The block exports the path directly instead of running `eval "$(phpz env)"`, which avoids starting a process on every new shell and cannot fail when `phpz` is missing from `PATH`.
- After a successful `phpz install`, phpz offers to run the same setup. The question is asked only once (recorded in `~/.phpz/.setup-prompted`), only when stdin and stderr are terminals, and not when `PATH` is already configured. Scripts and CI are never prompted.
- `phpz env` still prints the export line for shells that `setup` does not support.

`PATH` points to the stable `~/.phpz/current/bin`, so `phpz use` takes effect immediately in every open shell without re-evaluating anything. Only bash and zsh are supported; fish and PowerShell are not.

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

They operate on the `[PHP]` section of the active version's `php.ini`.

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
  paths.rs           ~/.phpz, versions directory, current link
  version.rs         version resolution and download URLs
  installed.rs       installed versions, active version, active php.ini path
  shell.rs           shell profile detection and the PATH block used by setup
  ini.rs             line-preserving php.ini editor
  commands/          one module per subcommand
  <module>/tests.rs  unit tests of <module>.rs (private items stay reachable)
tests/fixtures/      sample files used by the tests
```

Version and installed-version selection are pure functions (`select_version`, `select_installed`) so they are unit tested without network or disk access. CI (`.github/workflows/ci.yml`) runs build, tests and `clippy -D warnings`.

`phpz completions <shell>` prints a completion script generated from the command definitions.

---

## Cross-platform Support

- **Linux**: static tar.gz builds and symlinks. Supported.
- **macOS**: the same upstream provides builds for Intel and ARM64, and the code path is shared with Linux, but it has not been tested.
- **Windows**: official zip builds from windows.php.net and a shim system instead of symlinks. Not implemented. Today `install` fails with a clear "unsupported platform" error.

---

## Future Enhancements

- Windows support.
- Download integrity verification with pinned checksums.
- Optional compile-from-source install for custom extensions, which would also enable `phpz ext`.
- `php.ini` profiles and templates (dev, prod, testing).
- Global configuration shared across versions.
- Integration with Docker images for reproducibility.
