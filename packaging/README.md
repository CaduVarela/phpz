# Packaging

Recipes for installing phpz from a package manager. They point at the binaries of a GitHub release, with the SHA-256 from its `SHA256SUMS`. Each one has to be published in its own place:

| File | Where it goes |
|---|---|
| `scoop/phpz.json` | A Scoop bucket (a repository such as `scoop-phpz`): `scoop bucket add phpz <url>` and `scoop install phpz` |
| `homebrew/phpz.rb` | A Homebrew tap (a repository named `homebrew-phpz`, file in `Formula/`): `brew install CaduVarela/phpz/phpz` |
| `aur/PKGBUILD` | The AUR package `phpz-bin`. Run `makepkg --printsrcinfo > .SRCINFO` next to it before pushing |

For a new release, change the version and the hashes. Scoop can do it by itself (`checkver` and `autoupdate`).
