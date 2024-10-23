FROM archlinux

RUN pacman -Sy --noconfirm && \
    pacman -S  --noconfirm \
      libadwaita \
      rust \
      make \
      meson \
      pkgconf \
      webkitgtk-6.0 \
      gmime3

VOLUME [ "/src" ]
WORKDIR /src

#CMD [ "make", "docker" ]