PREFIX ?= /opt/birthday-reminders
BINDIR = $(PREFIX)/bin
CONFDIR = $(PREFIX)/etc
DATADIR = $(PREFIX)/data

BINARY = target/release/birthday-reminders
VERSION = $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
PACKAGE_NAME = birthday-reminders-$(VERSION)
PACKAGE_DIR = target/package/$(PACKAGE_NAME)

.PHONY: all build build-css install uninstall clean package-tar package-deb package-apk packages

all: build

build-css:
	npm run build:css

build: build-css
	cargo build --release

install: build
	install -d $(DESTDIR)$(BINDIR)
	install -d $(DESTDIR)$(CONFDIR)
	install -d $(DESTDIR)$(DATADIR)
	install -m 755 $(BINARY) $(DESTDIR)$(BINDIR)/birthday-reminders
	@if [ ! -f $(DESTDIR)$(CONFDIR)/config.yaml ]; then \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml; \
	else \
		echo "Config already exists, installing as config.yaml.new"; \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml.new; \
	fi
	rm -rf $(DESTDIR)$(PREFIX)/static
	@echo ""
	@echo "Installed to $(DESTDIR)$(PREFIX)"
	@echo "  Binary:     $(DESTDIR)$(BINDIR)/birthday-reminders"
	@echo "  Config:     $(DESTDIR)$(CONFDIR)/config.yaml"
	@echo "  Data:       $(DESTDIR)$(DATADIR)/"
	@echo "  Static:     (Embedded in binary)"

package-tar: build
	rm -rf $(PACKAGE_DIR)
	mkdir -p $(PACKAGE_DIR)
	cp $(BINARY) $(PACKAGE_DIR)/
	cp config.yaml.example $(PACKAGE_DIR)/
	cp -r package $(PACKAGE_DIR)/
	cp package/install.sh $(PACKAGE_DIR)/install.sh
	chmod +x $(PACKAGE_DIR)/install.sh
	cp package/uninstall.sh $(PACKAGE_DIR)/uninstall.sh
	chmod +x $(PACKAGE_DIR)/uninstall.sh
	tar -czf target/package/$(PACKAGE_NAME).tar.gz -C target/package $(PACKAGE_NAME)
	rm -rf $(PACKAGE_DIR)
	@echo ""
	@echo "Package created: target/package/$(PACKAGE_NAME).tar.gz"
	@echo "Copy to target server and run: tar xzf $(PACKAGE_NAME).tar.gz && cd $(PACKAGE_NAME) && ./install.sh"

package-deb: build
	@command -v nfpm >/dev/null 2>&1 || { echo "ERROR: nfpm not found. See https://nfpm.goreleaser.com/install/"; exit 1; }
	mkdir -p target/package
	cp $(BINARY) ./birthday-reminders
	VERSION=$(VERSION) nfpm package --packager deb -t target/package/
	rm -f ./birthday-reminders
	@echo ""
	@echo "Package created: target/package/birthday-reminders_$(VERSION)_amd64.deb"

package-apk: build-css
	@command -v nfpm >/dev/null 2>&1 || { echo "ERROR: nfpm not found. See https://nfpm.goreleaser.com/install/"; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "ERROR: docker not found, required for musl/Alpine build"; exit 1; }
	mkdir -p target/package
	docker build --target builder -t birthday-reminders-builder .
	docker create --name birthday-reminders-apk-extract birthday-reminders-builder
	docker cp birthday-reminders-apk-extract:/app/target/release/birthday-reminders ./birthday-reminders
	docker rm birthday-reminders-apk-extract
	VERSION=$(VERSION) nfpm package --packager apk -t target/package/
	rm -f ./birthday-reminders
	@echo ""
	@echo "Package created: target/package/birthday-reminders_$(VERSION)_amd64.apk"

packages: package-tar package-deb package-apk

# Uninstall everything installed by 'make install'.
uninstall:
	rm -f $(DESTDIR)$(BINDIR)/birthday-reminders
	rm -rf $(DESTDIR)$(MIGRATIONSDIR)
	rm -rf $(DESTDIR)$(PREFIX)/static
	rm -rf $(DESTDIR)$(DATADIR)
	if [ -d $(DESTDIR)$(CONFDIR) ]; then \
	  echo "NOTE: Config at $(DESTDIR)$(CONFDIR) was preserved. Remove manually if desired."; \
	fi
	@echo "Uninstall complete."

clean:
	cargo clean
