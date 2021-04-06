#!/usr/bin/bash

rm -Rf ./build
mkdir -p ./build

cp ../../moneta ../../config-dist.toml ../../zsh-completions ../../bash-completions ../moneta.service ../sysusers.d build
(cd build && tar cfz moneta.tar.gz .)

commit=$(git rev-parse --short HEAD)
checksum=$(cd build && md5sum moneta.tar.gz)
arch=$(uname -m)

sed -e "s/COMMIT/$commit/g" -e "s/ARCH/$arch/g" -e "s/CHECKSUM/${checksum%% *}/g" PKGBUILD > build/PKGBUILD
cp moneta.install build

(cd build && makepkg)
