#!/usr/bin/bash

rm -Rf build/moneta
mkdir -p build/moneta

mkdir -p build/moneta/DEBIAN
cp deb-control.txt build/moneta/DEBIAN/control

cp deb-postinst.sh build/moneta/DEBIAN/postinst
chmod 755 build/moneta/DEBIAN/postinst

cp deb-prerm.sh build/moneta/DEBIAN/prerm
chmod 755 build/moneta/DEBIAN/prerm

cp deb-preinst.sh build/moneta/DEBIAN/preinst
chmod 755 build/moneta/DEBIAN/preinst

mkdir -p build/moneta/usr/bin
cp ../../moneta build/moneta/usr/bin
chmod 755 build/moneta/usr/bin/moneta

mkdir -p build/moneta/etc/moneta
cp ../../config-dist.toml build/moneta/etc/moneta/config-dist.toml

mkdir -p build/moneta/etc/systemd/system
cp ../moneta.service build/moneta/etc/systemd/system/
cp ../moneta-backup.service build/moneta/etc/systemd/system/
cp ../moneta-backup.timer build/moneta/etc/systemd/system/

mkdir -p build/moneta/var/backups
cp backup.sh build/moneta/var/backups/backup-moneta.sh

mkdir -p build/moneta/usr/lib/sysusers.d
cp ../sysusers.d build/moneta/usr/lib/sysusers.d/moneta.conf

mkdir -p build/moneta/usr/share/zsh/vendor-completions
cp ../../zsh-completions build/moneta/usr/share/zsh/vendor-completions/_moneta

mkdir -p build/moneta/usr/share/bash-completion/completions
cp ../../zsh-completions build/moneta/usr/share/bash-completion/completions/moneta

(cd build && dpkg-deb --build moneta)
