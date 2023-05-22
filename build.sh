#!/bin/bash

[ ! -d uploads ] && mkdir uploads
[ ! -d bin ] && mkdir bin

root=$(pwd)
target_list=(x86_64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-unknown-linux-gnu aarch64-unknown-linux-musl)
for target in ${target_list[@]}; do

  if [[ $target == *"aarch64"* ]]; then
    arch=aarch64 bash +x unpack.sh
  else
    bash +x unpack.sh
  fi
  
  LD="/var/packages/pan-xunlei-com/target/host/lib"
  ARCH=""
  VERSION=(cat bin/version)
  if [[ $target == *"aarch64"* ]]; then
    ARCH="arm64"
    LD=$LD/ld-linux-aarch64.so.1
  else
    ARCH="amd64"
    LD=$LD/ld-linux-x86-64.so.2
  fi

  # default feature
  cargo zigbuild --release --target=$target
  upx --lzma target/$target/release/xunlei
  cargo deb --target=$target --no-build --no-strip
  cargo generate-rpm --target=$target --payload-compress none
  cd target/$target/release
  tar czvf xunlei-$tag-$target.tar.gz xunlei
  shasum -a 256 xunlei-$tag-$target.tar.gz >xunlei-$tag-$target.tar.gz.sha256
  mv xunlei-$tag-$target.tar.gz $root/uploads/
  mv xunlei-$tag-$target.tar.gz.sha256 $root/uploads/
  cd -
  cd target/$target/debian
  rename 's/.*/xunlei-'$tag'-'$target'.deb/' *.deb
  mv ./* $root/uploads/
  cd -
  cd target/$target/generate-rpm
  rename 's/.*/xunlei-'$tag'-'$target'.rpm/' xunlei*.rpm
  mv ./* $root/uploads/
  cd -

  # embed feature
  cargo zigbuild --release --target=$target --no-default-features --features embed
  upx --lzma target/$target/release/xunlei
  cargo deb --target=$target --no-build --no-strip
  cargo generate-rpm --target=$target --payload-compress none
  cd target/$target/release
  tar czvf xunlei-embed-$tag-$target.tar.gz xunlei
  shasum -a 256 xunlei-embed-$tag-$target.tar.gz >xunlei-embed-$tag-$target.tar.gz.sha256
  mv xunlei-embed-$tag-$target.tar.gz $root/uploads/
  mv xunlei-embed-$tag-$target.tar.gz.sha256 $root/uploads/
  cd -
  cd target/$target/debian
  rename 's/.*/xunlei-embed-'$tag'-'$target'.deb/' *.deb
  mv ./* $root/uploads/
  cd -
  cd target/$target/generate-rpm
  rename 's/.*/xunlei-embed-'$tag'-'$target'.rpm/' xunlei*.rpm
  mv ./* $root/uploads/
  cd -

  # embed feature patchelf
  if [[ $target == *"musl"* ]]; then
    cd bin
    patchelf --set-interpreter $LD xunlei-pan-cli-launcher.$ARCH
    patchelf --set-interpreter $LD xunlei-pan-cli-web
    patchelf --set-interpreter $LD xunlei-pan-cli.$VERSION.$ARCH
    cd -
    cargo zigbuild --release --target=$target --no-default-features --features embed,patch
    upx --lzma target/$target/release/xunlei
    cargo deb --target=$target --no-build --no-strip
    cargo generate-rpm --target=$target --payload-compress none
    cd target/$target/release
    tar czvf xunlei-embed-patch-$tag-$target.tar.gz xunlei
    shasum -a 256 xunlei-embed-patch-$tag-$target.tar.gz >xunlei-embed-patch-$tag-$target.tar.gz.sha256
    mv xunlei-embed-patch-$tag-$target.tar.gz $root/uploads/
    mv xunlei-embed-patch-$tag-$target.tar.gz.sha256 $root/uploads/
    cd -
    cd target/$target/debian
    rename 's/.*/xunlei-embed-patch-'$tag'-'$target'.deb/' *.deb
    mv ./* $root/uploads/
    cd -
    cd target/$target/generate-rpm
    rename 's/.*/xunlei-embed-patch-'$tag'-'$target'.rpm/' xunlei*.rpm
    mv ./* $root/uploads/
    cd -
  fi

  # launcher feature
  mkdir -p xunlei-launcher-$tag-$target/bin
  cp bin/* xunlei-launcher-$tag-$target/bin/
  cargo zigbuild --release --target=$target --no-default-features --features launcher
  upx --lzma target/$target/release/xunlei
  mv target/$target/release/xunlei xunlei-launcher-$tag-$target/
  tar -czvf xunlei-launcher-$tag-$target.tar.gz xunlei-launcher-$tag-$target/*
  shasum -a 256 xunlei-launcher-$tag-$target.tar.gz >xunlei-launcher-$tag-$target.tar.gz.sha256
  mv xunlei-launcher-$tag-$target.tar.gz uploads/
  mv xunlei-launcher-$tag-$target.tar.gz.sha256 uploads/
  rm -rf xunlei-launcher-$tag-$target

  # launcher feature patchelf
  if [[ $target == *"musl"* ]]; then
    mkdir -p xunlei-launcher-patch-$tag-$target/bin
    cp bin/* xunlei-launcher-patch-$tag-$target/bin/
    cd xunlei-launcher-patch-$tag-$target/bin/
    patchelf --set-interpreter $LD xunlei-pan-cli-launcher.$ARCH
    patchelf --set-interpreter $LD xunlei-pan-cli-web
    patchelf --set-interpreter $LD xunlei-pan-cli.$VERSION.$ARCH
    cd -
    cargo zigbuild --release --target=$target --no-default-features --features launcher,patch
    upx --lzma target/$target/release/xunlei
    mv target/$target/release/xunlei xunlei-launcher-patch-$tag-$target/
    tar -czvf xunlei-launcher-patch-$tag-$target.tar.gz xunlei-launcher-patch-$tag-$target/*
    shasum -a 256 xunlei-launcher-patch-$tag-$target.tar.gz >xunlei-launcher-patch-$tag-$target.tar.gz.sha256
    mv xunlei-launcher-patch-$tag-$target.tar.gz uploads/
    mv xunlei-launcher-patch-$tag-$target.tar.gz.sha256 uploads/
    rm -rf xunlei-launcher-patch-$tag-$target
  fi

  rm -r bin/*
  ls -lah uploads
done
