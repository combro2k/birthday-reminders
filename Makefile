PREFIX ?= /opt/birthday-reminders
BINDIR = $(PREFIX)/bin
CONFDIR = $(PREFIX)/etc
DATADIR = $(PREFIX)/data
STATICDIR = $(PREFIX)/static

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
	install -d $(DESTDIR)$(DATADIR)
	install -d $(DESTDIR)$(STATICDIR)
	install -m 755 $(BINARY) $(DESTDIR)$(BINDIR)/birthday-reminders
	@if [ ! -f $(DESTDIR)$(CONFDIR)/config.yaml ]; then \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml; \
	else \
		echo "Config already exists, installing as config.yaml.new"; \
		install -m 640 config.yaml.example $(DESTDIR)$(CONFDIR)/config.yaml.new; \
	fi
	cp -r static/* $(DESTDIR)$(STATICDIR)/
	@echo ""
	@echo "Installed to $(DESTDIR)$(PREFIX)"
	@echo "  Binary:     $(DESTDIR)$(BINDIR)/birthday-reminders"
	@echo "  Config:     $(DESTDIR)$(CONFDIR)/config.yaml"
	@echo "  Data:       $(DESTDIR)$(DATADIR)/"
	@echo "  Static:     $(DESTDIR)$(STATICDIR)/"

package: build
	rm -rf $(PACKAGE_DIR)
	mkdir -p $(PACKAGE_DIR)
	cp $(BINARY) $(PACKAGE_DIR)/
	cp config.yaml.example $(PACKAGE_DIR)/
	cp -r static $(PACKAGE_DIR)/
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

# Uninstall everything installed by 'make install'.
uninstall:
	rm -f $(DESTDIR)$(BINDIR)/birthday-reminders
	rm -rf $(DESTDIR)$(MIGRATIONSDIR)
	rm -rf $(DESTDIR)$(STATICDIR)
	rm -rf $(DESTDIR)$(DATADIR)
	if [ -d $(DESTDIR)$(CONFDIR) ]; then \
	  echo "NOTE: Config at $(DESTDIR)$(CONFDIR) was preserved. Remove manually if desired."; \
	fi
	@echo "Uninstall complete."

clean:
	cargo clean
