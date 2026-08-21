# Maintainer: <insert your name> <email>
# Build and package the checkout containing this PKGBUILD. Set BUILDDIR to a
# directory outside the checkout when invoking makepkg; this project already
# has its own src/ directory, which `makepkg -C` would otherwise remove.
# Use ./scripts/makepkg to create and clean up a safe temporary build directory.

pkgname=paru
pkgver=2.1.0+r70.ge553fc7
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

source=()
sha256sums=()

pkgver() {
  cd "$startdir"
  git describe --long --tags --abbrev=7 2>/dev/null \
    | sed 's/^v//;s/\([^-]*\)-\([^-]*\)-\(.*\)/\1+r\2.\3/' \
    || echo "$pkgver"
}

build() {
  cd "$startdir"
  cargo build --release --features "git,generate"
}

check() {
  cd "$startdir"
  cargo test --features "git,generate" -- --skip integration_tests 2>/dev/null
}

package() {
  cd "$startdir"

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
