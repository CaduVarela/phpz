# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `phpz use` without a version switches to the version the current folder pins.
- `phpz prune` removes the settings generated for projects that no longer exist (`--dry-run` only lists them).
- `ext install` compiles extensions from PECL on Linux, against a PHP compiled with `--build`.
- `install --build` compares the signing key with the PHP release managers' fingerprints built into phpz, and asks before building from a source signed by any other key.

## [0.3.0] - 2026-11-01

Per-project versions and extensions, a `php` shim, and more ways to get PHP on Linux and macOS.

### Added

- `phpz install <version> --build` (Linux) compiles PHP from the official source at php.net, checked against its SHA-256 and PGP signature. It gives the newest patch release (the prebuilt builds trail php.net) and builds `intl`, `soap` and `opcache` as `.so` files that `phpz ext enable|disable` can switch. Extensions whose development libraries are missing are left out with the package to install. `--jobs` limits the compile jobs, `--replace` compiles over a prebuilt version keeping its `php.ini`, and `phpz list` shows `[source]`. A failed build prints its first errors and keeps the log. PHP before 8.1 may not compile with OpenSSL 3, and macOS is not supported.
- Builds on Linux and macOS. `phpz variants` lists the static-php builds (`minimal`, `common`, `bulk`, `gnu-bulk`) with their size and what each has and lacks compared to the default. `phpz install <version> --variant <name>` picks one, `--replace` moves an installed version to another build keeping its `php.ini` and `conf.d`, and `phpz list --remote --variant <name>` lists what a build offers. `phpz list` shows a version's build when it is not `common`.
- `phpz ext enable <name>` on a build that lacks the extension says which build has it and what that build would lose, and switches if you answer yes. `phpz install` asks the same when your global extension settings need a larger build. The default build stays `common`, and a larger one is never installed without a yes.
- Extensions per project. The `[extensions]` table of a `.phpz.toml` (`enable`, `disable`, names only) applies inside that project through a generated `php.ini`, and nowhere else. `phpz ext sync --project` reports the extensions the version lacks and, on Windows with `--install`, downloads them. `phpz ext list` shows what the project changes.
- A `php` shim, so the plain `php` command follows the project. `phpz setup` puts `~/.phpz/shims` on `PATH`; the `php` in it is phpz, which starts the version for the current folder with that version's own `php.ini`. It also clears a `PHP_INI_SCAN_DIR` exported by another PHP (Scoop) on Windows, which fixes extensions loading twice.
- Per-project PHP versions. A `.php-version` (just a version) or a `.phpz.toml` (`php = "8.3"`, plus extensions later) in a folder or any parent folder sets the version there. `phpz local <version>` writes the file, `phpz local --unset` removes it, `phpz local` shows what applies, and `phpz which` prints the php that would run. `current`, `list`, `config` and `ext` follow the project. A pinned version that is not installed is an error, never a fallback.

### Changed

- `phpz setup` now puts the shims directory on `PATH` instead of the default version's directory. Run it once after upgrading from 0.2: it updates the marked block in `~/.bashrc` or `~/.zshrc`, or adds the entry to the Windows user `PATH`, without duplicating anything. `phpz setup --undo` removes both the new and the old entry.

## [0.2.0] - 2026-11-01

Adds Windows support and extension management.

### Added

- `phpz ext list`, `phpz ext enable <name>` and `phpz ext disable <name>`. On Windows they switch the extensions that come as DLLs in the official zip, editing only the right line of `php.ini`. On Linux and macOS they report what is built into the static builds.
- `phpz ext enable|disable <name> --global` keeps the choice in `~/.phpz/config.toml`. New installs apply it automatically and `phpz ext sync [--all]` applies it to installed versions. Extensions a version does not have are skipped with a message.
- `phpz ext install <name>` (Windows) downloads an extension that does not ship with PHP from the PECL server, puts its DLL in `ext` and enables it. The server publishes no checksums, so the first download asks for confirmation (refused in scripts unless `--allow-unverified`) and its SHA-256 is recorded in `downloads.lock`; later downloads of the same address must match. Packages that come with extra libraries, such as imagick, are refused for now.
- `phpz ext sync --install` downloads the wished extensions a version is missing. New installs and a plain `sync` never download; they suggest it.
- Windows support (x64). `phpz install` downloads the official non-thread-safe zip from windows.php.net, verifies its SHA-256 and writes a `php.ini` with `extension_dir` and a default set of extensions. `phpz use` switches versions with a directory junction, which needs no administrator rights.
- `phpz setup` and `phpz setup --undo` on Windows edit the user `PATH` in the registry, keeping the value's type and every other entry exactly as they were. `phpz env` prints a PowerShell line on Windows.
- PHP 7.0 to 7.3 on Windows (their final releases: 7.0.33, 7.1.33, 7.2.34, 7.3.33), with the `php.ini` naming each series expects.
- `phpz install` asks for confirmation before installing a Windows build that has no published checksum, and refuses in scripts unless `--allow-unverified` is passed. Linux and macOS do not ask for now.
- Older Windows patches (PHP 7.0 and newer) can be installed, for example `phpz install 8.3.12`, and are listed by `phpz list --remote --all`. They come from the windows.php.net archives. The SHA-256 is verified when the release has an SBOM; otherwise phpz says that no checksum is published.
- `phpz use` warns on Windows when `PHP_INI_SCAN_DIR` or `PHPRC` are set, since they make PHP read configuration from another installation.
- Downloads are verified against the published SHA-256 whenever the source provides one.
- `PHPZ_HOME` changes the directory phpz uses instead of `~/.phpz`.
- CI also builds and tests on Windows.

### Changed

- The question that follows the first install no longer accepts by default when the input is closed.

### Fixed

- Closing the output pipe early (`phpz list --remote | head -1`) no longer prints a panic message.

## [0.1.0] - 2026-10-31

First release. Linux only.

### Added

- `phpz install <version>` downloads a prebuilt static PHP CLI from static-php-cli. It accepts `8`, `8.3` or `8.3.9` and installs into `~/.phpz/versions/<version>` without touching the system PHP. Interrupted installs leave nothing behind.
- `phpz uninstall <version>` removes an installed version. It needs the exact version and refuses to remove the active one.
- `phpz use <version>` switches the active version through the `~/.phpz/current` symlink, replaced atomically. Partial versions pick the newest installed match.
- `phpz list` shows installed versions and marks the active one. `phpz list --remote` shows what can be downloaded, with `--all` for every patch.
- `phpz current` shows the active version, and `phpz reset` deactivates it.
- `phpz setup` adds phpz to `PATH` by editing `~/.bashrc` or `~/.zshrc` with a marked, idempotent block, and `phpz setup --undo` removes it again. After the first install, phpz offers to run it when it is in an interactive terminal.
- `phpz env` prints the `PATH` export for shells that `setup` does not support.
- `phpz config get|set|remove` reads and edits the active version's `php.ini`. Edits touch only the changed line and keep comments, ordering and repeated keys such as `extension=`.
- `phpz completions <shell>` prints a completion script for bash, zsh, fish, elvish or powershell.
- Each version has its own `php.ini` and `conf.d` directory, wired through a small wrapper script.
- CI workflow running build, tests and `clippy`.

### Known limitations

- Windows is not supported yet. macOS and aarch64 builds exist upstream but are untested.
- Downloads are checked for completeness only, not against a checksum, because upstream does not publish any.
- The static builds ship a fixed set of extensions and cannot load external ones such as `xdebug`, `intl` or `opcache`. `phpz ext` exits with an explanatory error for now.
- `phpz setup` supports bash and zsh only.
