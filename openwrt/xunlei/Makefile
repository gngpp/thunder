include $(TOPDIR)/rules.mk

PKG_NAME:=xunlei

PKG_VERSION:=3.8.0-17

PKG_LICENSE:=MIT
PKG_MAINTAINER:=gngpp <gngppz@gmail.com>

PKG_SOURCE:=xunlei-launch-$(PKG_VERSION)-$(ARCH)-unknown-linux-musl.tar.gz
PKG_SOURCE_URL:=https://github.com/gngpp/xunlei/releases/download/v$(PKG_VERSION)/
PKG_HASH:=skip
STRIP:=false

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  SECTION:=net
  CATEGORY:=Network
  SUBMENU:=Web Servers/Proxies
  TITLE:=xunlei
  DEPENDS:=@(aarch64||x86_64) +USE_GLIBC:libpthread +libgcc +libstdcpp
  URL:=https://nas.xunlei.com/
endef

define Package/$(PKG_NAME)/description
  Synology Nas Thunder runs on OpenWrt
endef

define Download/sha256sum
	FILE:=$(PKG_SOURCE).sha256
	URL_FILE:=$(FILE)
	URL:=$(PKG_SOURCE_URL)
	HASH:=skip
endef
$(eval $(call Download,sha256sum))

define Build/Prepare
	mv $(DL_DIR)/$(PKG_SOURCE).sha256 .
	cp $(DL_DIR)/$(PKG_SOURCE) .
	shasum -a 256 -c $(PKG_SOURCE).sha256
	rm $(PKG_SOURCE).sha256 $(PKG_SOURCE)

	tar -xvf $(DL_DIR)/$(PKG_SOURCE)
	mv xunlei-launcher-$(PKG_VERSION)-$(ARCH)-unknown-linux-musl/* $(PKG_BUILD_DIR)/
endef

define Build/Compile
	echo "nas-xunlei using precompiled binary."
endef

define Package/$(PKG_NAME)/postrm
#!/bin/sh
  rm -rf /usr/share/xunlei
  rm -rf /usr/syno/synoman/webman/modules
  exit 0
endef

define Package/$(PKG_NAME)/postinst
#!/bin/sh
  /etc/init.d/xunlei disable && /etc/init.d/xunlei enable
  exit 0	
endef

define Package/$(PKG_NAME)/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/xunlei $(1)/usr/bin/xunlei
	$(INSTALL_DIR) $(1)/usr/share/xunlei
	$(INSTALL_DIR) $(1)/usr/share/xunlei/target
	$(INSTALL_DIR) $(1)/usr/share/xunlei/target/host
	$(CP) $(PKG_BUILD_DIR)/bin/* $(1)/usr/share/xunlei/target/

	$(INSTALL_DIR) $(1)/usr/syno/synoman/webman/modules
	$(INSTALL_DIR) $(1)/usr/share/xunlei/target/host/etc
	$(INSTALL_DIR) $(1)/usr/share/xunlei/target/host/usr/syno/synoman/webman/modules
	$(INSTALL_BIN) $(CURDIR)/files/synoinfo.conf $(1)/usr/share/xunlei/target/host/etc/synoinfo.conf
	$(INSTALL_BIN) $(CURDIR)/files/authenticate.cgi $(1)/usr/share/xunlei/target/host/usr/syno/synoman/webman/modules/authenticate.cgi

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) $(CURDIR)/files/xunlei.init $(1)/etc/init.d/xunlei
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) $(CURDIR)/files/xunlei.config $(1)/etc/config/xunlei

ifeq ($(CONFIG_USE_MUSL),y)
	$(INSTALL_DIR) $(1)/usr/share/xunlei/target/host/exploit
	$(INSTALL_BIN) $(CURDIR)/exploit/libc.so.6 $(1)/usr/share/xunlei/target/host/exploit/libc.so.6
	$(INSTALL_BIN) $(CURDIR)/exploit/libdl.so.2 $(1)/usr/share/xunlei/target/host/exploit/libdl.so.2
	$(INSTALL_BIN) $(CURDIR)/exploit/libm.so.6 $(1)/usr/share/xunlei/target/host/exploit/libm.so.6
	$(INSTALL_BIN) $(CURDIR)/exploit/libstdc++.so.6 $(1)/usr/share/xunlei/target/host/exploit/libstdc++.so.6
	$(INSTALL_BIN) $(CURDIR)/exploit/libpthread.so.0 $(1)/usr/share/xunlei/target/host/exploit/libpthread.so.0
endif
endef

$(eval $(call BuildPackage,$(PKG_NAME)))
