# irlosd seat image: IrlosStudio (headless, NVENC) + irlosd on Ubuntu 24.04.
#   docker compose build

FROM ubuntu:24.04 AS base
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake ninja-build pkg-config ca-certificates \
      libglib2.0-dev libcurl4-openssl-dev libjansson-dev uthash-dev nlohmann-json3-dev \
      libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev \
      libswresample-dev libswscale-dev libx264-dev libmbedtls-dev libfdk-aac-dev \
      libffmpeg-nvenc-dev librist-dev libsrt-openssl-dev \
      libgl1-mesa-dev libgles2-mesa-dev libglvnd-dev libegl1 libdrm-dev libva-dev libpci-dev \
      libx11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xinerama0-dev \
      libxcb-composite0-dev libxcb-xfixes0-dev libxinerama-dev libxss-dev libxcomposite-dev \
      libxdamage-dev libxkbcommon-dev libwayland-dev \
      libpulse-dev libasound2-dev libjack-jackd2-dev libpipewire-0.3-dev libspeexdsp-dev \
      libfontconfig-dev libfreetype6-dev libudev-dev libv4l-dev \
      qt6-base-dev qt6-base-private-dev libqt6svg6-dev qt6-qpa-plugins \
      ffmpeg fonts-dejavu-core tini \
 && rm -rf /var/lib/apt/lists/* \
 && cat /proc/sys/kernel/random/uuid | tr -d - > /etc/machine-id

FROM base AS studio
COPY IrlosStudio /src
RUN cmake -S /src -B /build -G Ninja \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo \
      -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_INSTALL_LIBDIR=lib/x86_64-linux-gnu \
      -DOBS_VERSION_OVERRIDE=30.2.3 \
      -DENABLE_BROWSER=OFF -DENABLE_WEBSOCKET=OFF -DENABLE_NEW_MPEGTS_OUTPUT=OFF \
      -DENABLE_SCRIPTING=OFF -DENABLE_VLC=OFF -DENABLE_AJA=OFF -DENABLE_WEBRTC=OFF \
      -DENABLE_QSV11=OFF -DENABLE_SERVICE_UPDATES=OFF \
 && cmake --build /build \
 && DESTDIR=/out cmake --install /build --strip

FROM rust:1-bookworm AS irlosd
WORKDIR /irlosd
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --release --locked

FROM base
# The BRB/test card (moving pattern, silent audio) and the weak-signal banner.
RUN mkdir -p /usr/share/irlosd && cd /usr/share/irlosd \
 && font=/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf \
 && ffmpeg -loglevel error -f lavfi -i "testsrc2=size=1920x1080:rate=30" -f lavfi -i "anullsrc=r=48000:cl=stereo" -t 10 \
      -vf "drawtext=fontfile=$font:text='BE RIGHT BACK':fontsize=110:fontcolor=white:box=1:boxcolor=black@0.65:boxborderw=40:x=(w-tw)/2:y=h-th-130" \
      -c:v libx264 -preset veryfast -pix_fmt yuv420p -g 30 -c:a aac -shortest brb.mp4 \
 && ffmpeg -loglevel error -f lavfi -i "color=c=black@0.0:size=1920x1080,format=rgba" -frames:v 1 \
      -vf "drawtext=fontfile=$font:text='WEAK SIGNAL':fontsize=72:fontcolor=white:box=1:boxcolor=0xC0392B@0.9:boxborderw=28:x=(w-tw)/2:y=48" \
      low.png
COPY --from=studio /out/ /
COPY studio /usr/share/irlosd/studio
COPY --from=irlosd /irlosd/target/release/irlosd /usr/local/bin/irlosd
ENTRYPOINT ["/usr/bin/tini", "--", "irlosd"]
CMD ["serve"]
