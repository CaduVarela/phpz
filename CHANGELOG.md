# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Windows support (x64). `phpz install` downloads the official non-thread-safe zip from windows.php.net, verifies its SHA-256 and writes a `php.ini` with `extension_dir` and a default set of extensions. `phpz use` switches versions with a directory junction, which needs no administrator rights.
- `phpz setup` and `phpz setup --undo` on Windows edit the user `PATH` in the registry, keeping the value's type and every other entry exactly as they were. `phpz env` prints a PowerShell line on Windows.
- Older Windows patches (PHP 7.4 and newer) can be installed, for example `phpz install 8.3.12`, and are listed by `phpz list --remote --all`. They come from the windows.php.net archives. The SHA-256 is verified when the release has an SBOM; otherwise phpz says that no checksum is published.
- `phpz use` warns on Windows when `PHP_INI_SCAN_DIR` or `PHPRC` are set, since they make PHP read configuration from another installation.
- Downloads are verified against the published SHA-256 whenever the source provides one.
- `PHPZ_HOME` changes the directory phpz uses instead of `~/.phpz`.
- CI also builds and tests on Windows.

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
