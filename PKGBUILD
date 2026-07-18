# Maintainer: Refined7075 <yxgw5rdy2@mozmail.com>
pkgname=acord-bin
pkgver=6.0.2
pkgrel=1
pkgdesc="An alternative Discord client aimed and lower-spec or storage-sensitive PCs that supports themes, plugins, and more!"
arch=('x86_64')
url="https://github.com/daniyusk/Acord"
license=('GPL3')
depends=('libayatana-appindicator' 'webkit2gtk' 'gtk3' 'gst-plugins-good')
provides=('acord')
conflicts=('acord')
source=("https://github.com/daniyusk/Acord/releases/download/v${pkgver}/acord_${pkgver}_amd64.deb")
sha256sums=('e29cb66b447ba6a733e5aded5bb768ab4678f3eeb32c254da79117699ba05967')

package() {
    bsdtar -xf "$srcdir/data.tar.gz" -C "$pkgdir"
}
