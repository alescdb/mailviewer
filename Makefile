CURRENT_DIR := $(shell pwd)
BUILD_DIR   := _build
DEBUG       := $(CURRENT_DIR)/dist
EXECUTABLE  := $(CURRENT_DIR)/dist/bin/mailviewer
SOURCES     := $(wildcard src/**.rs src/**.ui src/**.css src/config.rs.in)
RESOURCES   := $(DEBUG)/share/mailviewer/mailviewer.gresource
SCHEMAS     := $(DEBUG)/dist/share/glib-2.0/schemas/gschemas.compiled
MANIFEST    := $(CURRENT_DIR)/io.github.alescdb.mailviewer.json

all: gresources
	cargo build

run: gresources
	cargo run -- ../sample.eml

format:
	cargo +nightly fmt

$(RESOURCES): $(CURRENT_DIR)/src/mailviewer.gresource.xml src/**.ui
	mkdir -p $(DEBUG)/share/mailviewer
	glib-compile-resources \
		--sourcedir=$(CURRENT_DIR)/src \
		--target=$(DEBUG)/share/mailviewer/mailviewer.gresource \
		$(CURRENT_DIR)/src/mailviewer.gresource.xml

$(SCHEMAS): $(CURRENT_DIR)/data/io.github.alescdb.mailviewer.gschema.xml 
	mkdir -p $(DEBUG)/share/glib-2.0/schemas
	glib-compile-schemas \
		--targetdir=$(DEBUG)/share/glib-2.0/schemas/ \
		$(CURRENT_DIR)/data/

gresources: $(RESOURCES) $(SCHEMAS)

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

install: clean
	meson setup --strip --buildtype release $(BUILD_DIR)
	meson install -C $(BUILD_DIR)

reconfigure:
	meson setup $(BUILD_DIR) --reconfigure --prefix=$(DEBUG)

$(BUILD_DIR):
	meson setup $(BUILD_DIR) --prefix=$(DEBUG)

$(EXECUTABLE): $(BUILD_DIR) $(SOURCES)
	meson compile -C $(BUILD_DIR) 

clean:
	rm -rf $(BUILD_DIR) $(DEBUG) target buildir .flatpak .flatpak-builder .repo

.PHONY: all format build reconfigure flatpak-run install clean $(BUILD_DIR)