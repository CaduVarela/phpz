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
| Windows | Not implemented yet |

---

## Installation

phpz is built from source with a recent [Rust toolchain](https://rustup.rs):

```bash
git clone https://github.com/CaduVarela/phpz.git
cd phpz
cargo build --release
cp target/release/phpz ~/.local/bin/   # or any directory on your PATH
```

Then let phpz put the active PHP version on your `PATH`:

```bash
phpz setup
```

This adds a small marked block to your `~/.bashrc` (bash) or `~/.zshrc` (zsh), and running it twice is harmless. phpz also offers to do it right after your first `phpz install`, when it runs in an interactive terminal. Open a new terminal afterwards. `phpz setup --undo` removes the block and leaves the rest of the file untouched. Other shells are not supported yet: add `export PATH="$HOME/.phpz/current/bin:$PATH"` to your shell profile by hand.

Optionally enable shell completion (bash example):

```bash
phpz completions bash > ~/.local/share/bash-completion/completions/phpz
```

---

## Usage

```bash
phpz list --remote      # list versions available for download (add --all for every patch)
phpz install 8.3        # install the latest 8.3.x (also accepts "8" or "8.3.9")
phpz list               # list installed versions, marking the active one
phpz uninstall 8.3.32   # remove an installed version (exact version, not the active one)
phpz use 8.3            # switch the active version
phpz current            # show the active version
phpz reset              # deactivate phpz's PHP (removes the "current" link)
phpz setup              # add phpz to your PATH (bash and zsh)
phpz setup --undo       # remove it from your shell profile again

phpz config get memory_limit
phpz config set memory_limit 512M
phpz config remove memory_limit

phpz completions bash   # also zsh, fish, elvish and powershell
```

`phpz config` edits the `php.ini` of the active version. Only the line you change is touched: comments, ordering and repeated keys such as `extension=` are preserved. Values are written exactly as given, so quote them yourself when you need a string (`phpz config set error_log '"/tmp/php errors.log"'`).

---

## How it works

PHP builds come from [static-php-cli](https://static-php.dev) (`dl.static-php.dev`), which publishes prebuilt, dependency-free `php` binaries. Installing a version is a download and an extraction, with no compiler and no system libraries involved.

Each version lives in its own directory with its own `php.ini`:

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

See [docs/DESIGN.md](docs/DESIGN.md) for the design and its trade-offs.

---

## Limitations

- The binaries are fully static. They ship with a fixed set of extensions (bcmath, curl, gd, mbstring, mysqlnd, openssl, pdo_mysql, pdo_pgsql, pdo_sqlite, redis, zip and others), and extensions that are not compiled in (for example `xdebug`, `intl` or `opcache`) cannot be loaded.
- Downloads are not verified against a checksum yet, because upstream does not publish them.

---

## Roadmap

- Windows support, using the official builds from windows.php.net.
- Download integrity verification.
- Extension management (`phpz ext`), pending support for builds that can load external extensions. The command exists but currently exits with an explanatory error.
- `php.ini` profiles (`phpz config use dev`).

---

## License

MIT, see [LICENSE](LICENSE).
