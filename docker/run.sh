#!/bin/bash

if [ ! -d "/var/packages/pan-xunlei-com/target/var" ]; then
    id=$(cat /proc/sys/kernel/random/uuid | cut -c1-7)
    echo "unique=\"synology_${id}_720+\"" > /var/packages/pan-xunlei-com/target/host/etc/synoinfo.conf
fi
mkdir -p /usr/syno/synoman/webman/modules
ln -s /var/packages/pan-xunlei-com/target/host/etc/synoinfo.conf /etc/synoinfo.conf
ln -s /var/packages/pan-xunlei-com/target/host/usr/syno/synoman/webman/modules/authenticate.cgi /usr/syno/synoman/webman/modules/authenticate.cgi

mount --bind /bin /rootfs/bin
mount --bind /run /rootfs/run
mount --bind /lib /rootfs/lib
mount --bind /usr /rootfs/usr
mount --bind /mnt /rootfs/mnt
mount --bind /etc /rootfs/etc
mount --bind /sbin /rootfs/sbin
mount --bind /dev /rootfs/dev
mount --bind /var /rootfs/var
mount --bind /tmp /rootfs/tmp
mount --bind /root /rootfs/root
mount --bind /proc /rootfs/proc
mount --bind /opt/data /rootfs/opt/data
mount --bind /downloads /rootfs/downloads

chroot /rootfs /bin/bash -c "echo 'nameserver 119.29.29.29' > /etc/resolv.conf && /bin/mount -t proc none /proc && /bin/xunlei launcher -c /opt/data -d /downloads"

exec "$@"
