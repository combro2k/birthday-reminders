PREFIX ?= /opt/birthday-reminders
BINDIR = $(PREFIX)/bin
CONFDIR = $(PREFIX)/etc
DATADIR = $(PREFIX)/share
MIGRATIONSDIR = $(DATADIR)/migrations
STATICDIR = $(DATADIR)/static

BINARY = target/release/birthday-reminders

.PHONY: all build install uninstall clean

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

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/birthday-reminders
	rm -rf $(DESTDIR)$(MIGRATIONSDIR)
	rm -rf $(DESTDIR)$(STATICDIR)
	@echo "NOTE: Config at $(DESTDIR)$(CONFDIR) was preserved. Remove manually if desired."

clean:
	cargo clean
