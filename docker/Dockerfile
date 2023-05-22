FROM alpine:latest as builder

ARG VERSION
ARG TARGETPLATFORM
ARG BUILDPLATFORM

RUN if [ "${TARGETPLATFORM}" = "linux/arm64" ]; then \
        echo "aarch64" > arch; \
    elif [ "${TARGETPLATFORM}" = "linux/amd64" ]; then \
        echo "x86_64" > arch; \
    fi
RUN apk update && apk add wget
RUN mkdir -p /var/packages/pan-xunlei-com/target
RUN wget https://github.com/gngpp/xunlei/releases/download/v${VERSION}/xunlei-launcher-${VERSION}-$(cat arch)-unknown-linux-musl.tar.gz
RUN tar -xvf xunlei-launcher-${VERSION}-$(cat arch)-unknown-linux-musl.tar.gz && chmod 755 /var/packages/pan-xunlei-com/target
RUN mv xunlei-launcher-${VERSION}-$(cat arch)-unknown-linux-musl/bin/* /var/packages/pan-xunlei-com/target/
RUN mv xunlei-launcher-${VERSION}-$(cat arch)-unknown-linux-musl/xunlei .
COPY authenticate.cgi /var/packages/pan-xunlei-com/target/host/usr/syno/synoman/webman/modules/authenticate.cgi
COPY synoinfo.conf /var/packages/pan-xunlei-com/target/host/etc/synoinfo.conf
RUN chmod 755 /var/packages/pan-xunlei-com/target/host/usr/syno/synoman/webman/modules/authenticate.cgi
RUN chmod 755 /var/packages/pan-xunlei-com/target/host/etc/synoinfo.conf

FROM alpine:latest
LABEL org.opencontainers.image.source https://github.com/gngpp/xunlei
WORKDIR /rootfs

ENV LANG=C.UTF-8 DEBIAN_FRONTEND=noninteractive LANG=zh_CN.UTF-8 LANGUAGE=zh_CN.UTF-8 LC_ALL=C

RUN apk update && apk add bash
RUN mkdir -p /rootfs/bin /rootfs/run \
    /rootfs/lib /rootfs/proc \
    /rootfs/usr /rootfs/mnt \
    /rootfs/etc /rootfs/sbin \
    /rootfs/sys /rootfs/dev \
    /rootfs/var /rootfs/tmp \
    /rootfs/root /rootfs/boot \
    /rootfs/opt/data /rootfs/downloads \
    /opt/data /downloads

COPY run.sh /bin/run.sh
RUN chmod +x /bin/run.sh

COPY --from=builder /var/packages/pan-xunlei-com/target /var/packages/pan-xunlei-com/target
COPY --from=builder /xunlei /bin/xunlei

VOLUME [ "/opt/data", "/downloads" ]
ENTRYPOINT ["/bin/run.sh"]