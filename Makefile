PREFIX ?= /opt/birthday-reminders
BINDIR = $(PREFIX)/bin
CONFDIR = $(PREFIX)/etc
DATADIR = $(PREFIX)/share
MIGRATIONSDIR = $(DATADIR)/migrations
STATICDIR = $(DATADIR)/static

BINARY = target/release/birthday-reminders
VERSION = $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
PACKAGE_NAME = birthday-reminders-$(VERSION)
PACKAGE_DIR = target/package/$(PACKAGE_NAME)

.PHONY: all build install uninstall clean package

all: build

build:
	cargo build --release

install: build
	install -d $(DESTDIR)$(BINDIR)
	install -d $(DESTDIR)$(CONFDIR)
	install -d $(DESTDIR)$(MIGRATIONSDIR)
	install -d $(DESTDIR)$(STATICDIR)
	install -m 755 $(BINARY) $(DESTDIR)$(BINDIR)/birthday-reminders
	@if [ ! -f $(DESTDIR)$(CONFDIR)/config.yaml ]; then \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml; \
	else \
		echo "Config already exists, installing as config.yaml.new"; \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml.new; \
	fi
	cp -r migrations/* $(DESTDIR)$(MIGRATIONSDIR)/
	cp -r static/* $(DESTDIR)$(STATICDIR)/
	@echo ""
	@echo "Installed to $(DESTDIR)$(PREFIX)"
	@echo "  Binary:     $(DESTDIR)$(BINDIR)/birthday-reminders"
	@echo "  Config:     $(DESTDIR)$(CONFDIR)/config.yaml"
	@echo "  Migrations: $(DESTDIR)$(MIGRATIONSDIR)/"
	@echo "  Static:     $(DESTDIR)$(STATICDIR)/"

package: build
	rm -rf $(PACKAGE_DIR)
	mkdir -p $(PACKAGE_DIR)
	cp $(BINARY) $(PACKAGE_DIR)/
	cp config.yaml.example $(PACKAGE_DIR)/
	cp -r migrations $(PACKAGE_DIR)/
	cp -r static $(PACKAGE_DIR)/
	cp -r package $(PACKAGE_DIR)/
	cp package/install.sh $(PACKAGE_DIR)/install.sh
	chmod +x $(PACKAGE_DIR)/install.sh
	tar -czf target/package/$(PACKAGE_NAME).tar.gz -C target/package $(PACKAGE_NAME)
	@echo ""
	@echo "Package created: target/package/$(PACKAGE_NAME).tar.gz"
	@echo "Copy to target server and run: tar xzf $(PACKAGE_NAME).tar.gz && cd $(PACKAGE_NAME) && ./install.sh"

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/birthday-reminders
	rm -rf $(DESTDIR)$(MIGRATIONSDIR)
	rm -rf $(DESTDIR)$(STATICDIR)
	@echo "NOTE: Config at $(DESTDIR)$(CONFDIR) was preserved. Remove manually if desired."

clean:
	cargo clean
