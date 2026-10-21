# qeyes Makefile - Cross-Platform Build, Test, Lint, Package & Install

BINARY_NAME := qeyes
APP_NAME    := QEyes
VERSION     ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "0.1.0")
CARGO       ?= cargo
CROSS       ?= cross
PREFIX      ?= /usr/local
DESTDIR     ?=
DIST_DIR    := dist
UNAME_S     := $(shell uname -s)
APPLICATIONS_DIR ?= $(DESTDIR)/Applications

# Target triples for all supported platforms
TARGET_LINUX_X86_64   := x86_64-unknown-linux-gnu
TARGET_LINUX_ARM64    := aarch64-unknown-linux-gnu
TARGET_MACOS_X86_64   := x86_64-apple-darwin
TARGET_MACOS_ARM64    := aarch64-apple-darwin
TARGET_WINDOWS_X86_64 := x86_64-pc-windows-msvc
TARGET_WINDOWS_ARM64  := aarch64-pc-windows-msvc

.PHONY: all build app dev run fmt lint check test test-xvfb clean \
        build-linux-x86_64 build-linux-arm64 build-all-linux \
        build-macos-x86_64 build-macos-arm64 build-macos-app build-macos-universal \
        build-windows-x86_64 build-windows-arm64 \
        install uninstall package package-linux package-macos package-windows help

all: build

help:
	@echo "qeyes ($(VERSION)) Makefile targets:"
	@echo "  make build                 Build native release binary"
	@echo "  make app                   Build local macOS $(APP_NAME).app bundle"
	@echo "  make dev                   Run qeyes in debug mode (pass ARGS='...')"
	@echo "  make fmt                   Format Rust code with rustfmt"
	@echo "  make lint                  Run rustfmt check and clippy (-D warnings)"
	@echo "  make test                  Run unit, golden-render, and CLI parity tests"
	@echo "  make test-xvfb             Run full test suite under Xvfb (Linux X11)"
	@echo "  make build-linux-x86_64    Build Linux x86_64 release binary"
	@echo "  make build-linux-arm64     Build Linux arm64 (aarch64) release binary"
	@echo "  make build-macos-x86_64    Build macOS Intel (x86_64) release binary"
	@echo "  make build-macos-arm64     Build macOS Apple Silicon (arm64) release binary"
	@echo "  make build-macos-app       Build local macOS $(APP_NAME).app bundle with icon"
	@echo "  make build-macos-universal Build macOS Universal binary and $(APP_NAME).app bundle"
	@echo "  make build-windows-x86_64  Build Windows x86_64 release binary"
	@echo "  make build-windows-arm64   Build Windows arm64 release binary"
	@echo "  make package               Package release for current host platform"
	@echo "  make package-linux         Package Linux .tar.gz archive (TARGET=...)"
	@echo "  make package-macos         Package macOS $(APP_NAME).app bundle and installable .dmg"
	@echo "  make package-windows       Package Windows .zip archive (TARGET=...)"
	@echo "  make install               Install qeyes binary (and $(APP_NAME).app on macOS)"
	@echo "  make uninstall             Remove installed qeyes files"
	@echo "  make clean                 Clean cargo build artifacts and dist/"

build:
	$(CARGO) build --release --locked

app: build-macos-app

dev run:
	$(CARGO) run -- $(ARGS)

fmt:
	$(CARGO) fmt --all

lint check:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --all-targets -- -D warnings

test:
	$(CARGO) test --all-targets --locked

test-xvfb:
	@if command -v xvfb-run >/dev/null 2>&1; then \
		xvfb-run -a -s "-screen 0 1280x800x24 -noreset" $(CARGO) test --all-targets --locked; \
	else \
		echo "xvfb-run not found; running headless tests only"; \
		$(CARGO) test --all-targets --locked; \
	fi

# ---------------------------------------------------------------------------
# Platform-specific build targets
# ---------------------------------------------------------------------------

build-linux-x86_64:
	$(CARGO) build --release --locked --target $(TARGET_LINUX_X86_64)

build-linux-arm64:
	@if [ "$$(uname -m)" = "aarch64" ] || [ "$$(uname -m)" = "arm64" ]; then \
		$(CARGO) build --release --locked --target $(TARGET_LINUX_ARM64); \
	elif command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then \
		CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
		$(CARGO) build --release --locked --target $(TARGET_LINUX_ARM64); \
	elif command -v $(CROSS) >/dev/null 2>&1; then \
		$(CROSS) build --release --locked --target $(TARGET_LINUX_ARM64); \
	else \
		$(CARGO) build --release --locked --target $(TARGET_LINUX_ARM64); \
	fi

build-all-linux: build-linux-x86_64 build-linux-arm64

build-macos-x86_64:
	$(CARGO) build --release --locked --target $(TARGET_MACOS_X86_64)

build-macos-arm64:
	$(CARGO) build --release --locked --target $(TARGET_MACOS_ARM64)

assets/qeyes.icns: assets/qeyes.png
	@if command -v iconutil >/dev/null 2>&1 && command -v sips >/dev/null 2>&1; then \
		echo "Generating assets/qeyes.icns from assets/qeyes.png..."; \
		ICONSET="assets/qeyes.iconset"; \
		mkdir -p "$$ICONSET"; \
		sips -z 16 16 assets/qeyes.png --out "$$ICONSET/icon_16x16.png" >/dev/null; \
		sips -z 32 32 assets/qeyes.png --out "$$ICONSET/icon_16x16@2x.png" >/dev/null; \
		sips -z 32 32 assets/qeyes.png --out "$$ICONSET/icon_32x32.png" >/dev/null; \
		sips -z 64 64 assets/qeyes.png --out "$$ICONSET/icon_32x32@2x.png" >/dev/null; \
		sips -z 128 128 assets/qeyes.png --out "$$ICONSET/icon_128x128.png" >/dev/null; \
		sips -z 256 256 assets/qeyes.png --out "$$ICONSET/icon_128x128@2x.png" >/dev/null; \
		sips -z 256 256 assets/qeyes.png --out "$$ICONSET/icon_256x256.png" >/dev/null; \
		sips -z 512 512 assets/qeyes.png --out "$$ICONSET/icon_256x256@2x.png" >/dev/null; \
		sips -z 512 512 assets/qeyes.png --out "$$ICONSET/icon_512x512.png" >/dev/null; \
		sips -z 1024 1024 assets/qeyes.png --out "$$ICONSET/icon_512x512@2x.png" >/dev/null; \
		iconutil -c icns "$$ICONSET" -o assets/qeyes.icns; \
		rm -rf "$$ICONSET"; \
	fi

build-macos-app: build assets/qeyes.icns
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS"
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources"
	cp "target/release/$(BINARY_NAME)" "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"
	chmod 755 "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"
	cp assets/Info.plist "$(DIST_DIR)/$(APP_NAME).app/Contents/Info.plist"
	@if [ -f assets/qeyes.icns ]; then \
		cp assets/qeyes.icns "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources/qeyes.icns"; \
	fi
	@if command -v codesign >/dev/null 2>&1; then \
		echo "Ad-hoc code signing $(APP_NAME).app..."; \
		codesign --force --deep -s - "$(DIST_DIR)/$(APP_NAME).app"; \
	fi
	@echo "Successfully built $(DIST_DIR)/$(APP_NAME).app"

build-macos-universal: assets/qeyes.icns
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS"
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources"
	@HAS_X86=0; HAS_ARM=0; \
	if $(CARGO) build --release --locked --target $(TARGET_MACOS_X86_64) 2>/dev/null; then HAS_X86=1; fi; \
	if $(CARGO) build --release --locked --target $(TARGET_MACOS_ARM64) 2>/dev/null; then HAS_ARM=1; fi; \
	if [ "$$HAS_X86" = "1" ] && [ "$$HAS_ARM" = "1" ]; then \
		echo "Combining x86_64 and arm64 binaries with lipo..."; \
		lipo -create \
			"target/$(TARGET_MACOS_X86_64)/release/$(BINARY_NAME)" \
			"target/$(TARGET_MACOS_ARM64)/release/$(BINARY_NAME)" \
			-output "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"; \
	elif [ "$$HAS_ARM" = "1" ]; then \
		echo "Target $(TARGET_MACOS_X86_64) not available; bundling arm64 binary..."; \
		cp "target/$(TARGET_MACOS_ARM64)/release/$(BINARY_NAME)" "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"; \
	elif [ "$$HAS_X86" = "1" ]; then \
		echo "Target $(TARGET_MACOS_ARM64) not available; bundling x86_64 binary..."; \
		cp "target/$(TARGET_MACOS_X86_64)/release/$(BINARY_NAME)" "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"; \
	else \
		echo "Cross targets not installed; building host release binary..."; \
		$(CARGO) build --release --locked; \
		cp "target/release/$(BINARY_NAME)" "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"; \
	fi
	chmod 755 "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"
	cp assets/Info.plist "$(DIST_DIR)/$(APP_NAME).app/Contents/Info.plist"
	@if [ -f assets/qeyes.icns ]; then \
		cp assets/qeyes.icns "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources/qeyes.icns"; \
	fi
	@if command -v codesign >/dev/null 2>&1; then \
		echo "Ad-hoc code signing $(APP_NAME).app..."; \
		codesign --force --deep -s - "$(DIST_DIR)/$(APP_NAME).app"; \
	fi
	@echo "Successfully built $(DIST_DIR)/$(APP_NAME).app"

build-windows-x86_64:
	$(CARGO) build --release --locked --target $(TARGET_WINDOWS_X86_64)

build-windows-arm64:
	$(CARGO) build --release --locked --target $(TARGET_WINDOWS_ARM64)

# ---------------------------------------------------------------------------
# Packaging targets
# ---------------------------------------------------------------------------

package:
ifeq ($(UNAME_S),Darwin)
	$(MAKE) package-macos
else ifeq ($(OS),Windows_NT)
	$(MAKE) package-windows
else
	$(MAKE) package-linux
endif

package-linux:
	@TARGET_TRIPLE=$${TARGET:-$(TARGET_LINUX_X86_64)}; \
	BIN_PATH="target/$$TARGET_TRIPLE/release/$(BINARY_NAME)"; \
	if [ ! -f "$$BIN_PATH" ]; then BIN_PATH="target/release/$(BINARY_NAME)"; fi; \
	STAGE_DIR="$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE"; \
	mkdir -p "$$STAGE_DIR"; \
	cp "$$BIN_PATH" "$$STAGE_DIR/$(BINARY_NAME)"; \
	cp README.md LICENSE "$$STAGE_DIR/"; \
	cp docs/qeyes.1 "$$STAGE_DIR/"; \
	cp assets/qeyes.desktop assets/qeyes.svg "$$STAGE_DIR/"; \
	tar -C "$(DIST_DIR)" -czf "$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE.tar.gz" \
		"$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE"; \
	rm -rf "$$STAGE_DIR"

package-macos: build-macos-universal
	@mkdir -p "$(DIST_DIR)"
	tar -C "$(DIST_DIR)" -czf "$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.app.tar.gz" "$(APP_NAME).app"
	@if command -v hdiutil >/dev/null 2>&1; then \
		STAGE_DIR="$(DIST_DIR)/dmg-staging"; \
		rm -rf "$$STAGE_DIR"; \
		mkdir -p "$$STAGE_DIR"; \
		cp -R "$(DIST_DIR)/$(APP_NAME).app" "$$STAGE_DIR/"; \
		ln -s /Applications "$$STAGE_DIR/Applications"; \
		rm -f "$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.dmg"; \
		hdiutil create -volname "$(APP_NAME)" \
			-srcfolder "$$STAGE_DIR" \
			-ov -format UDZO \
			"$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.dmg"; \
		rm -rf "$$STAGE_DIR"; \
		echo "Created $(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.dmg"; \
	fi

package-windows:
	@TARGET_TRIPLE=$${TARGET:-$(TARGET_WINDOWS_X86_64)}; \
	BIN_PATH="target/$$TARGET_TRIPLE/release/$(BINARY_NAME).exe"; \
	if [ ! -f "$$BIN_PATH" ]; then BIN_PATH="target/release/$(BINARY_NAME).exe"; fi; \
	STAGE_DIR="$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE"; \
	mkdir -p "$$STAGE_DIR"; \
	cp "$$BIN_PATH" "$$STAGE_DIR/$(BINARY_NAME).exe"; \
	cp README.md LICENSE "$$STAGE_DIR/"; \
	if command -v 7z >/dev/null 2>&1; then \
		(cd "$(DIST_DIR)" && 7z a -tzip "$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE.zip" "$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE"); \
	elif command -v zip >/dev/null 2>&1; then \
		(cd "$(DIST_DIR)" && zip -r "$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE.zip" "$(BINARY_NAME)-$(VERSION)-$$TARGET_TRIPLE"); \
	fi; \
	rm -rf "$$STAGE_DIR"

# ---------------------------------------------------------------------------
# Install / Uninstall targets
# ---------------------------------------------------------------------------

ifeq ($(UNAME_S),Darwin)
install: build-macos-app
	@mkdir -p "$(APPLICATIONS_DIR)"
	rm -rf "$(APPLICATIONS_DIR)/$(APP_NAME).app"
	cp -R "$(DIST_DIR)/$(APP_NAME).app" "$(APPLICATIONS_DIR)/"
	@echo "Installed $(APP_NAME).app to $(APPLICATIONS_DIR)"
	@if [ -w "$(DESTDIR)$(PREFIX)/bin" ] || [ "$$(id -u)" = "0" ]; then \
		install -d "$(DESTDIR)$(PREFIX)/bin"; \
		install -m 755 "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)" "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"; \
		install -d "$(DESTDIR)$(PREFIX)/share/man/man1"; \
		install -m 644 "docs/qeyes.1" "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"; \
		echo "Installed CLI binary and man page to $(PREFIX)"; \
	else \
		echo "Note: $(DESTDIR)$(PREFIX)/bin is not writable without sudo; skipped CLI binary install."; \
	fi

uninstall:
	rm -rf "$(APPLICATIONS_DIR)/$(APP_NAME).app"
	rm -f "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"
	rm -f "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"
	@echo "Uninstalled $(APP_NAME)"
else
install: build
	install -d "$(DESTDIR)$(PREFIX)/bin"
	install -m 755 "target/release/$(BINARY_NAME)" "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"
	install -d "$(DESTDIR)$(PREFIX)/share/man/man1"
	install -m 644 "docs/qeyes.1" "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"
	install -d "$(DESTDIR)$(PREFIX)/share/applications"
	install -m 644 "assets/qeyes.desktop" "$(DESTDIR)$(PREFIX)/share/applications/qeyes.desktop"
	install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps"
	install -m 644 "assets/qeyes.svg" "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/qeyes.svg"
	@for sz in 16 32 48 64 128 256; do \
		install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/$${sz}x$${sz}/apps"; \
		install -m 644 "assets/qeyes-$${sz}.png" "$(DESTDIR)$(PREFIX)/share/icons/hicolor/$${sz}x$${sz}/apps/qeyes.png"; \
	done
	install -d "$(DESTDIR)$(PREFIX)/share/pixmaps"
	install -m 644 "assets/qeyes-256.png" "$(DESTDIR)$(PREFIX)/share/pixmaps/qeyes.png"

uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"
	rm -f "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"
	rm -f "$(DESTDIR)$(PREFIX)/share/applications/qeyes.desktop"
	rm -f "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/qeyes.svg"
	@for sz in 16 32 48 64 128 256; do \
		rm -f "$(DESTDIR)$(PREFIX)/share/icons/hicolor/$${sz}x$${sz}/apps/qeyes.png"; \
	done
	rm -f "$(DESTDIR)$(PREFIX)/share/pixmaps/qeyes.png"
endif

clean:
	$(CARGO) clean
	rm -rf "$(DIST_DIR)"
