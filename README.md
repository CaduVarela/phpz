<h1 align="center">phpz</h1>

<p align="center">A PHP version manager with extensions that follow your projects.</p>

<p align="center">
  <a href="https://github.com/CaduVarela/phpz/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/CaduVarela/phpz/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Linux, macOS and Windows" src="https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-lightgrey.svg">
</p>

```bash
phpz install 8.3          # no sudo, no compiler
phpz use 8.3              # switch the default
phpz local 8.2            # or pin this folder to another version
phpz ext enable intl      # extensions switch like versions do
```

Everything lives in `~/.phpz`. The PHP your system or web server uses is never touched.

## One file per project

```toml
# .phpz.toml
php = "8.3"

[extensions]
enable = ["intl", "redis"]
disable = ["xdebug"]
```

Run `php` inside that folder and you get PHP 8.3 with those extensions. Everywhere else nothing changes.

## Commands

| | |
|---|---|
| `phpz list --remote` | versions you can install |
| `phpz install <version>` | install a version (`--build` compiles it on Linux) |
| `phpz use [version]` | set the default, or the one this folder pins |
| `phpz local <version>` | pin the current folder |
| `phpz ext list \| enable \| disable \| install` | manage extensions |
| `phpz config set <key> <value>` | edit one `php.ini` setting |
| `phpz current` | what applies here |

The full list is in the [guide](docs/GUIDE.md).

## Install

```bash
git clone https://github.com/CaduVarela/phpz.git && cd phpz
cargo install --path . && phpz setup
```

Linux and Windows are tested. macOS builds and passes the tests, but nobody has installed PHP with it yet.

[Guide](docs/GUIDE.md) · [Design](docs/DESIGN.md) · [Changelog](CHANGELOG.md) · MIT license
