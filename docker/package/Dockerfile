FROM archlinux/base

RUN echo "Server = https://mirrors.kernel.org/archlinux/\$repo/os/\$arch" > /etc/pacman.d/mirrorlist
RUN pacman -Syyu --noconfirm && pacman -S base-devel --noconfirm && pacman -S dpkg git --noconfirm && rm -Rf /var/cache
