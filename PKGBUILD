# Maintainer: <insert your name> <email>
# Build from local checkout. Copy this PKGBUILD to a separate directory
# (e.g. /tmp/paru-build), adjust PARU_SOURCE below, and run makepkg.
# The pkg/ and src/ directories created by makepkg will stay outside
# the original repo root.

pkgname=paru
pkgver=2.1.0+r68.gd168c28
pkgrel=1
pkgdesc="Feature packed AUR helper"
arch=('x86_64')
url="https://github.com/Morganamilo/paru"
license=('GPL-3.0-or-later')
depends=('pacman' 'git')
makedepends=('cargo')
optdepends=('bat: colored pkgbuild printing'
            'devtools: build in chroot and downloading pkgbuilds')
provides=('paru')
conflicts=('paru')

# ── Point this at your local checkout ─────────────────────────
# Change the path to match where you cloned paru.
_PARU_SOURCE=/home/ripdog/git-clones/paru

source=("$pkgname::git+file://${_PARU_SOURCE}")
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/$pkgname"
  git describe --long --tags --abbrev=7 2>/dev/null \
    | sed 's/^v//;s/\([^-]*\)-\([^-]*\)-\(.*\)/\1+r\2.\3/' \
    || echo "$pkgver"
}

build() {
  cd "$srcdir/$pkgname"
  cargo build --release --features "git,generate"
}

check() {
  cd "$srcdir/$pkgname"
  cargo test --features "git,generate" -- --skip integration_tests 2>/dev/null
}

package() {
  cd "$srcdir/$pkgname"

  # binary
  install -Dm0755 target/release/paru "$pkgdir/usr/bin/paru"

  # default config
  install -Dm0644 paru.conf "$pkgdir/etc/paru.conf"

  # shell completions
  install -Dm0644 completions/bash \
    "$pkgdir/usr/share/bash-completion/completions/paru"
  install -Dm0644 completions/fish \
    "$pkgdir/usr/share/fish/vendor_completions.d/paru.fish"
  install -Dm0644 completions/zsh \
    "$pkgdir/usr/share/zsh/site-functions/_paru"

  # man pages
  install -Dm0644 man/paru.8 \
    "$pkgdir/usr/share/man/man8/paru.8"
  install -Dm0644 man/paru.conf.5 \
    "$pkgdir/usr/share/man/man5/paru.conf.5"

  # license
  install -Dm0644 LICENSE \
    "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
