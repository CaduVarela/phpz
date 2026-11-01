# phpz guide

The full reference. For a short introduction see the [README](../README.md); for the design and its trade-offs see [DESIGN.md](DESIGN.md).

---

## Installation

phpz is built from source with [rustup](https://rustup.rs). The repository pins the Rust version in `rust-toolchain.toml`, so inside it `cargo` uses that version (run `rustup toolchain install` once if it is not installed yet; your other projects keep their own toolchain):

```bash
git clone https://github.com/CaduVarela/phpz.git
cd phpz
cargo install --path .   # installs to ~/.cargo/bin
```

Then let phpz put the active PHP version on your `PATH`:

```bash
phpz setup
```

**On Windows**, run the same commands in PowerShell. `phpz setup` adds `%USERPROFILE%\.phpz\shims` to your *user* `PATH` (in the registry, keeping the value's type and every other entry as they were), so open a new terminal afterwards, or run `phpz env | Invoke-Expression` to apply it to the current one. `phpz setup --undo` takes it out again.

**On Linux and macOS**, this adds a small marked block to your `~/.bashrc` (bash) or `~/.zshrc` (zsh), and running it twice is harmless. phpz also offers to do it right after your first `phpz install`, when it runs in an interactive terminal. To apply it right away, run `source ~/.bashrc` (or `source ~/.zshrc`) in the current terminal, or just open a new one. `phpz setup --undo` removes the block and leaves the rest of the file untouched. Other shells are not supported yet: add `export PATH="$HOME/.phpz/current/bin:$PATH"` to your shell profile by hand.

Optionally enable shell completion (bash example):

```bash
phpz completions bash > ~/.local/share/bash-completion/completions/phpz
```

---

## Usage

```bash
phpz list --remote      # list versions available for download (add --all for every patch)
phpz variants           # Linux and macOS: the builds you can choose from, and what each has
phpz install 8.3 --build                         # Linux: compile it from the official source (a few minutes)
phpz install 8.3 --variant bulk                  # a larger build with intl, opcache, imagick...
phpz install 8.3.32 --variant bulk --replace     # move an installed version to another build, keeping its php.ini
phpz install            # in a project: install the pinned version and the extensions it enables
phpz install 8.3        # install the latest 8.3.x (also accepts "8" or "8.3.9")
phpz install 7.3.33 --allow-unverified   # Windows: skip the question for a build with no published checksum
phpz list               # list installed versions, marking the active one
phpz uninstall 8.3.32   # remove an installed version (exact version, not the active one)
phpz use 8.3            # switch the active version
phpz use                 # switch to the version this folder pins
phpz prune               # remove generated files of projects that no longer exist
phpz self-update         # update phpz itself (--check only looks, --version X picks a release)
phpz current            # show the version that applies in the current directory
phpz local 8.2          # use PHP 8.2 in this folder and below (writes .php-version)
phpz local              # show what applies here, and where it comes from
phpz local --unset      # remove the .php-version of this folder
phpz which              # path of the php that runs here
phpz reset              # deactivate phpz's PHP (removes the "current" link)
phpz setup              # add phpz to your PATH (bash and zsh, or the user PATH on Windows)
phpz setup --undo       # remove it again
phpz env                # print the line that puts phpz on PATH for the current shell

phpz config get memory_limit
phpz config set memory_limit 512M
phpz config remove memory_limit

phpz ext list                    # what the active version has: enabled, disabled, built in
phpz ext enable intl             # switch an extension on in the active version
phpz ext disable gd              # and off again
phpz ext enable intl --global    # also keep it on in every version, now and when installed later
phpz ext install xdebug          # download (Windows) or compile (Linux) an extension that does not ship with PHP, and enable it
phpz ext sync --all              # apply those global choices to every installed version
phpz ext sync --install --all    # Windows: also download what each version is missing

phpz completions bash   # also zsh, fish, elvish and powershell
```

**Extensions.** On Windows the official zip ships the extensions as DLLs, so `phpz ext enable|disable` just switches the right line of the version's `php.ini` (the file keeps its comments, order and line endings). `--global` also records the choice in `~/.phpz/config.toml`, and from then on every version you install gets it, and `phpz ext sync` applies it to the ones you already have. An extension a version does not have is skipped with a message, never an error. On Linux and macOS the extensions are part of the build you installed: `phpz ext list` shows what is built in, and enabling a built-in one is a no-op. Asking for one the build lacks does not just fail: if another build has it, phpz says which, **says what that build would lose**, and switches only if you answer yes (see Builds below). 

**Installing extensions that do not ship with PHP.** `phpz ext install xdebug` downloads the newest stable release that has a build for your PHP (non thread safe, x64) from the PECL server at downloads.php.net, puts its DLL in the version's `ext` directory and enables it. `--version` picks another release and `--global` also keeps it wished for in every version. That server publishes no checksums, so the first download of a file asks for your confirmation (the default is no; in a script it is refused unless you pass `--allow-unverified`), and phpz records its SHA-256 in `~/.phpz/downloads.lock`. From then on the same address must give the same file, or nothing is installed. `phpz ext sync --install` downloads the wished extensions a version is missing, which is how a freshly installed PHP catches up. Installing a version and `phpz ext sync` without `--install` never download extensions by themselves. Extensions that come with extra libraries (imagick, for example) are refused for now. On Linux, a PHP compiled with `--build` compiles the extension from its PECL source with the `phpize` it keeps (it needs a compiler, `make` and `autoconf`), under the same consent and lock rules. Prebuilt static builds cannot load extra extensions.

**Builds on Linux and macOS.** static-php publishes several builds of PHP, and `phpz variants` shows each with its size and how its extensions differ from the default:

| Build | Size (PHP 8.3) | What it is |
|---|---|---|
| `minimal` | about 3 MB | a handful of core extensions |
| `common` | about 12 MB | the **default**: the usual web stack, fast to install |
| `bulk` | about 30 MB | adds `intl`, `opcache`, `imagick`, `apcu`, `swoole` and more, but lacks `pdo_sqlite` and `pdo_pgsql`, which `common` has |
| `gnu-bulk` | about 40 MB | the same extensions as `bulk`, built dynamically for glibc, so it can load `.so` files (Linux only, PHP 8.1 and newer) |

The lists come from the server when you ask, so they are never out of date. A larger build is never installed unless you ask for it with `--variant`, or answer yes to a question: phpz asks when your global extension settings need something the default build lacks, and when `phpz ext enable` finds that another build has the extension. The question always says what the other build lacks. `phpz install <version> --variant <name> --replace` moves a version to another build and keeps its `php.ini` and `conf.d`; the old build is put back if anything fails. `phpz list` shows the build of a version when it is not the default.

**Compiling from the official source (Linux).** `phpz install 8.3 --build` downloads the source from php.net, checks its SHA-256 and its PGP signature, and compiles it on your machine (about 4 minutes on 12 cores; `--jobs N` limits that). Nothing is hosted by anyone: it is php.net's own file, built by you. Why you might want it:

- **The newest patch release.** The prebuilt builds trail php.net (they had 8.3.32 when php.net had 8.3.36). Compiling gives you the release that has the latest security fixes.
- **Extensions you can switch.** `intl`, `soap` and `opcache` are built as `.so` files, so `phpz ext enable|disable` works on them like it does on Windows. `intl` and `soap` start enabled, and `opcache` starts off. (From PHP 8.5 `opcache` is part of PHP itself, so there is no file to switch and `phpz ext list` shows it as built in.)

It needs a C compiler, `make` and, for each extension, its development library, which `pkg-config` finds. What your machine lacks is not an error: that extension is left out, and phpz says which package to install (for example `libicu-dev` for `intl`). The result sits next to the other versions, `phpz list` marks it `[source]`, and `--replace` compiles over a prebuilt version of the same number while keeping its `php.ini`. A failed build prints the first errors and keeps only the log.

Not everything compiles: PHP 7.4 fails with the OpenSSL 3 that current distributions ship, and older series are not covered by the recipe at all. macOS is not supported yet.

**Per-project versions.** A project can say which PHP it needs. phpz looks for a `.php-version` (just a version such as `8.2`, `8.2.34` or `v8.2`, the same file other version managers read) or a `.phpz.toml` in the current folder and then in each parent folder, and uses the nearest one. `.phpz.toml` holds `php = "8.2"` and, later, the project's extensions, and wins over a `.php-version` in the same folder. A version that is not installed is an error that names the file and says what to run, never a silent fallback to another PHP. `phpz current`, `list`, `which`, `config` and `ext` follow the project, and so does the plain `php` command: `phpz setup` puts `~/.phpz/shims` (on Windows, `%USERPROFILE%\.phpz\shims`) on your `PATH`, and the `php` in it is phpz itself, which finds the version for the folder it runs in and starts that PHP. It starts it with that version's own `php.ini` and, on Windows, without a `PHP_INI_SCAN_DIR` exported by another PHP such as Scoop's, which would otherwise make extensions load twice. Arguments and the exit code pass through unchanged.

**Per-project extensions.** A `.phpz.toml` can also choose extensions for the project:

```toml
php = "8.3"

[extensions]
enable = ["intl", "redis"]
disable = ["xdebug"]
```

They apply only inside the project: when `php` runs there, phpz gives it a copy of the version's `php.ini` with those choices applied (kept under `~/.phpz/projects` and refreshed when the version's `php.ini` or the choices change), and the version's own `php.ini` is never touched. Other folders and other projects do not see them. An extension the version does not have is skipped quietly while `php` runs; `phpz ext sync --project` lists what the version lacks and, on Windows with `--install`, gets it under the same checksum rules as `phpz ext install`. `phpz ext list` shows what the project changes. The lists accept extension names only, never paths.

**Upgrading from 0.2.** The old setup put the default version on `PATH`, which cannot follow projects. Run `phpz setup` once: it updates the marked block (or the Windows `PATH` entry) in place, and `phpz use` reminds you while the old one is still in effect.

`phpz config` edits the `php.ini` of the version that applies here. Only the line you change is touched: comments, ordering and repeated keys such as `extension=` are preserved. Values are written exactly as given, so quote them yourself when you need a string (`phpz config set error_log '"/tmp/php errors.log"'`).

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

See [DESIGN.md](DESIGN.md) for the design and its trade-offs.

---

## Limitations

- On Linux and macOS each build has a fixed set of extensions. The default `common` build has the usual web stack; `intl`, `opcache` and `imagick` need `bulk` or `gnu-bulk`. `xdebug`, `mongodb`, `pcov`, `igbinary` and `memcached` are in none of them, and only `gnu-bulk` could load an extension file, which phpz cannot provide yet.
- On Linux and macOS, prebuilt downloads are not verified against a checksum, because upstream does not publish them. A version compiled with `--build` is verified against php.net's SHA-256 and PGP signature.
- On Windows, PHP 7.0 and newer are available (`phpz list --remote --all`), as the non-thread-safe x64 build, and the official builds need the Microsoft Visual C++ Redistributable. PHP 7.0 to 7.3 only have their final release (7.0.33, 7.1.33, 7.2.34, 7.3.33) in the official archive.
- On Windows, the newest release of each series and the most recent patches are verified against a published SHA-256. For most older patches windows.php.net publishes no checksum at all. Installing one asks for confirmation (the default is no), and in a script it is refused unless you pass `--allow-unverified`. Linux and macOS builds have no checksum either and do not ask for now; phpz only prints a note.
- On Windows, `PHP_INI_SCAN_DIR` or `PHPRC` set in your environment (for example by Scoop's PHP) make every PHP read configuration from outside its own directory, which can load extensions twice. The `php` shim clears them for the PHP it starts. A `php.exe` started directly, or a terminal that has not been set up with `phpz setup`, is not protected, and `phpz use` warns about them and says how to clear them.
- Windows builds the `PATH` of a program as the system `PATH` followed by the user `PATH`, so a PHP that sits on the *system* `PATH` (an XAMPP install, for example) is found before phpz's.

---

## Roadmap

- Download integrity verification for the prebuilt builds on Linux and macOS.
- Configure recipes for extensions that need extra libraries (imagick, for example) when compiling on Linux.
- macOS testing.
- `php.ini` profiles (`phpz config use dev`).

---

## License

MIT, see [LICENSE](../LICENSE).
