CURRENT_DIR := $(shell pwd)
BUILD_DIR   := _build
DEBUG       := $(CURRENT_DIR)/dist
EXECUTABLE  := $(CURRENT_DIR)/dist/bin/mailviewer
SOURCES     := $(wildcard src/*.rs src/*.ui)
RESOURCES   := $(DEBUG)/share/mailviewer/mailviewer.gresource

all: gresources
	cargo build

run:
	cargo run -- sample.eml

#all: build
#	GSETTINGS_SCHEMA_DIR=$(DEBUG)/share/glib-2.0/schemas \
#	RUST_LOG=mailviewer=debug \
#	$(EXECUTABLE) sample.eml

format:
	cargo +nightly fmt

gresources:
	mkdir -p $(DEBUG)/share/glib-2.0/schemas $(DEBUG)/share/mailviewer
	glib-compile-schemas \
		--targetdir=$(DEBUG)/share/glib-2.0/schemas/ \
		$(CURRENT_DIR)/data/
	glib-compile-resources \
		--sourcedir=$(CURRENT_DIR)/src \
		--target=$(DEBUG)/share/mailviewer/mailviewer.gresource \
		$(CURRENT_DIR)/src/mailviewer.gresource.xml

icon:
	mkdir -p ~/.icons
	cp $(CURRENT_DIR)/data/icons/hicolor/scalable/apps/org.cosinus.mailviewer.svg ~/.icons/

build: $(EXECUTABLE)
	meson install -C $(BUILD_DIR)

reconfigure:
	meson setup $(BUILD_DIR) --reconfigure --prefix=$(DEBUG)

$(BUILD_DIR):	
	meson setup $(BUILD_DIR) --prefix=$(DEBUG)

$(EXECUTABLE): $(BUILD_DIR) $(SOURCES)
	meson compile -C $(BUILD_DIR) 

clean:
	rm -rf $(BUILD_DIR) $(DEBUG) target buildir .flatpak .flatpak-builder .repo

.PHONY: all format build reconfigure $(BUILD_DIR)