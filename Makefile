CURRENT_DIR := $(shell pwd)
BUILD_DIR   := _build
DEBUG       := $(CURRENT_DIR)/dist
EXECUTABLE  := $(CURRENT_DIR)/dist/bin/mailviewer
SOURCES     := $(wildcard src/**.rs src/**.ui src/**.css src/config.rs.in)
RESOURCES   := $(DEBUG)/share/mailviewer/mailviewer.gresource
SCHEMAS     := $(DEBUG)/dist/share/glib-2.0/schemas/gschemas.compiled
MANIFEST    := $(CURRENT_DIR)/io.github.alescdb.mailviewer.json

all:
	cargo build

run:
	cargo run -- sample.eml

format:
	cargo +nightly fmt

flatpak: $(SOURCES) $(MANIFEST)
	flatpak run org.flatpak.Builder \
		--force-clean \
		--user \
		--install \
		--install-deps-from=flathub \
		--ccache \
		--mirror-screenshots-url=https://dl.flathub.org/media/ \
		--repo=repo \
		builddir $(MANIFEST)
	
flatpak-run:
	RUST_LOG=mailviewer=debug flatpak run io.github.alescdb.mailviewer sample.eml

icon:
	mkdir -p ~/.icons
	cp $(CURRENT_DIR)/data/icons/hicolor/scalable/apps/io.github.alescdb.mailviewer.svg ~/.icons/

build: $(EXECUTABLE)
	meson install -C $(BUILD_DIR)

po:
	meson compile mailviewer-update-po -C $(BUILD_DIR)

run-fr: build po
	RUST_LOG="mailviewer=debug" \
	GSETTINGS_SCHEMA_DIR="$(DEBUG)/share/glib-2.0/schemas" \
	LC_ALL="fr_FR.UTF-8" $(EXECUTABLE) sample.eml 

install: clean
	meson setup --strip --buildtype release $(BUILD_DIR)
	meson install -C $(BUILD_DIR)

reconfigure:
	meson setup $(BUILD_DIR) --reconfigure --prefix=$(DEBUG)

$(BUILD_DIR):
	if [ -d $(BUILD_DIR) ]; then \
		meson setup $(BUILD_DIR) --reconfigure --prefix=$(DEBUG); \
	else \
		meson setup $(BUILD_DIR) --prefix=$(DEBUG); \
	fi

$(EXECUTABLE): $(BUILD_DIR) $(SOURCES)
	meson compile -C $(BUILD_DIR) 

targets: $(BUILD_DIR)
	cd $(BUILD_DIR) && \
	meson introspect --targets | jq -r '.[].name'

clean:
	rm -rf $(BUILD_DIR) $(DEBUG) target buildir .flatpak .flatpak-builder .repo .venv flatpak-cargo-generator.py

.PHONY: all format build reconfigure flatpak-run install clean po $(BUILD_DIR)