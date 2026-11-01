# phpz

**phpz** is a simple and lightweight PHP version manager, inspired by tools like [nvm](https://github.com/nvm-sh/nvm).
It is in **active development** and not ready for production yet.

It manages multiple **PHP CLI versions** without touching the system-wide PHP used by Apache, Nginx, or other web servers. phpz only controls what it installs, inside `~/.phpz`.

---

## Why phpz?

- Existing PHP installers often bind to system-wide configuration.
- Switching between PHP versions can be messy.
- phpz gives you a developer-friendly way to install, switch and configure PHP versions without `sudo` and without build dependencies.

---

## Platform support

| Platform | Status |
|---|---|
| Linux (x86_64, aarch64) | Supported and tested |
| macOS (x86_64, aarch64) | Builds exist upstream, but untested |
| Windows (x64) | Supported and tested on Windows 11 |

---

## Installation

phpz is built from source with a recent [Rust toolchain](https://rustup.rs):

```bash
git clone https://github.com/CaduVarela/phpz.git
cd phpz
cargo install --path .   # installs to ~/.cargo/bin
```

Then let phpz put the active PHP version on your `PATH`:

```bash
phpz setup
```

**On Windows**, run the same commands in PowerShell. `phpz setup` adds `%USERPROFILE%\.phpz\current` to your *user* `PATH` (in the registry, keeping the value's type and every other entry as they were), so open a new terminal afterwards, or run `phpz env | Invoke-Expression` to apply it to the current one. `phpz setup --undo` takes it out again.

**On Linux and macOS**, this adds a small marked block to your `~/.bashrc` (bash) or `~/.zshrc` (zsh), and running it twice is harmless. phpz also offers to do it right after your first `phpz install`, when it runs in an interactive terminal. To apply it right away, run `source ~/.bashrc` (or `source ~/.zshrc`) in the current terminal, or just open a new one. `phpz setup --undo` removes the block and leaves the rest of the file untouched. Other shells are not supported yet: add `export PATH="$HOME/.phpz/current/bin:$PATH"` to your shell profile by hand.

Optionally enable shell completion (bash example):

```bash
phpz completions bash > ~/.local/share/bash-completion/completions/phpz
```

---

## Usage

```bash
phpz list --remote      # list versions available for download (add --all for every patch)
phpz install 8.3        # install the latest 8.3.x (also accepts "8" or "8.3.9")
phpz install 7.3.33 --allow-unverified   # Windows: skip the question for a build with no published checksum
phpz list               # list installed versions, marking the active one
phpz uninstall 8.3.32   # remove an installed version (exact version, not the active one)
phpz use 8.3            # switch the active version
phpz current            # show the active version
phpz reset              # deactivate phpz's PHP (removes the "current" link)
phpz setup              # add phpz to your PATH (bash and zsh, or the user PATH on Windows)
phpz setup --undo       # remove it again
phpz env                # print the line that puts phpz on PATH for the current shell

phpz config get memory_limit
phpz config set memory_limit 512M
phpz config remove memory_limit

phpz completions bash   # also zsh, fish, elvish and powershell
```

`phpz config` edits the `php.ini` of the active version. Only the line you change is touched: comments, ordering and repeated keys such as `extension=` are preserved. Values are written exactly as given, so quote them yourself when you need a string (`phpz config set error_log '"/tmp/php errors.log"'`).

---

## How it works

On **Linux and macOS**, PHP builds come from [static-php-cli](https://static-php.dev) (`dl.static-php.dev`), which publishes prebuilt, dependency-free `php` binaries. On **Windows**, they are the official zips from [windows.php.net](https://windows.php.net/download/), checked against the SHA-256 published next to them. Installing a version is a download and an extraction, with no compiler and no system libraries involved.

Each version lives in its own directory with its own `php.ini`. On Linux and macOS:

```
~/.phpz/
  versions/
    8.3.32/
      bin/php        wrapper that points PHP at this version's php.ini
      libexec/php    the real static binary
      php.ini
      conf.d/
  current -> versions/8.3.32
```

On Windows the official zip is extracted as it comes, and `current` is a directory junction (no administrator rights needed):

```
%USERPROFILE%\.phpz\
  versions\
    8.4.27\
      php.exe, DLLs, ext\
      php.ini
  current  ->  versions\8.4.27
```

Set `PHPZ_HOME` to keep everything in another directory instead of `~/.phpz`.

See [docs/DESIGN.md](docs/DESIGN.md) for the design and its trade-offs.

---

## Limitations

- On Linux and macOS the binaries are fully static. They ship with a fixed set of extensions (bcmath, curl, gd, mbstring, mysqlnd, openssl, pdo_mysql, pdo_pgsql, pdo_sqlite, redis, zip and others), and extensions that are not compiled in (for example `xdebug`, `intl` or `opcache`) cannot be loaded.
- On Linux and macOS, downloads are not verified against a checksum, because upstream does not publish them.
- On Windows, PHP 7.0 and newer are available (`phpz list --remote --all`), as the non-thread-safe x64 build, and the official builds need the Microsoft Visual C++ Redistributable. PHP 7.0 to 7.3 only have their final release (7.0.33, 7.1.33, 7.2.34, 7.3.33) in the official archive.
- On Windows, the newest release of each series and the most recent patches are verified against a published SHA-256. For most older patches windows.php.net publishes no checksum at all. Installing one asks for confirmation (the default is no), and in a script it is refused unless you pass `--allow-unverified`. Linux and macOS builds have no checksum either and do not ask for now; phpz only prints a note.
- On Windows, `PHP_INI_SCAN_DIR` or `PHPRC` set in your environment (for example by Scoop's PHP) make every PHP read configuration from outside its own directory, which can load extensions twice. `phpz use` warns about them and says how to clear them.
- Windows builds the `PATH` of a program as the system `PATH` followed by the user `PATH`, so a PHP that sits on the *system* `PATH` (an XAMPP install, for example) is found before phpz's.

---

## Roadmap

- Download integrity verification on Linux and macOS.
- Extension management (`phpz ext`), pending support for builds that can load external extensions. The command exists but currently exits with an explanatory error.
- `php.ini` profiles (`phpz config use dev`).

---

## License

MIT, see [LICENSE](LICENSE).
