# phpz

A fast, lightweight PHP version manager. Like [nvm](https://github.com/nvm-sh/nvm), but for PHP, and with **extensions that follow your versions and your projects**.

```bash
phpz install 8.3        # no sudo, no compiler, no system libraries
phpz use 8.3
phpz ext enable intl    # switch extensions on and off like you switch versions
```

## Why phpz

- **Nothing system-wide.** Everything lives in `~/.phpz`. Apache, Nginx and the PHP your distribution ships are never touched.
- **Fast installs.** A version is a download and an extraction: static builds on Linux and macOS, the official zips on Windows (checked against their SHA-256).
- **Per-project versions.** Drop a `.php-version` or a `.phpz.toml` in a project and the plain `php` command follows it.
- **Per-project extensions.** Choose them in the same file, without editing any `php.ini`:

  ```toml
  php = "8.3"

  [extensions]
  enable = ["intl", "redis"]
  disable = ["xdebug"]
  ```

- **Extensions you can install.** `phpz ext install xdebug` downloads it on Windows and compiles it on Linux.
- **Your own `php.ini` per version.** `phpz config set memory_limit 512M` edits one line and keeps your comments.
- **Newest releases when you want them.** `phpz install 8.3 --build` compiles php.net's own source on your machine.

## Install

phpz is built with [rustup](https://rustup.rs). The repository pins the Rust version.

```bash
git clone https://github.com/CaduVarela/phpz.git
cd phpz
cargo install --path .
phpz setup              # puts the php shim on your PATH
```

## Everyday commands

```bash
phpz list --remote      # versions you can install
phpz install 8.3        # install the latest 8.3.x
phpz use 8.3            # make it the default
phpz local 8.2          # use 8.2 in this folder and below (writes .php-version)
phpz current            # what applies here
phpz ext list           # extensions of the active version
phpz ext enable intl    # switch one on
phpz config set memory_limit 512M
```

## Platforms

| Platform | Status |
|---|---|
| Linux (x86_64, aarch64) | Supported and tested |
| Windows (x64) | Supported and tested on Windows 11 |
| macOS (x86_64, aarch64) | Builds exist upstream, not tested yet |

## More

- [Guide](docs/GUIDE.md): every command, builds and variants, compiling from source, extensions, limitations.
- [Design](docs/DESIGN.md): how it works and why.
- [Changelog](CHANGELOG.md).

phpz is in active development and not ready for production yet.

## License

MIT, see [LICENSE](LICENSE).
