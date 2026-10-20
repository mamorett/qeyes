# qeyes Makefile - Cross-Platform Build, Test, Lint, Package & Install

BINARY_NAME := qeyes
APP_NAME    := QEyes
VERSION     ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "0.1.0")
CARGO       ?= cargo
CROSS       ?= cross
PREFIX      ?= /usr/local
DESTDIR     ?=
DIST_DIR    := dist

# Target triples for all supported platforms
TARGET_LINUX_X86_64   := x86_64-unknown-linux-gnu
TARGET_LINUX_ARM64    := aarch64-unknown-linux-gnu
TARGET_MACOS_X86_64   := x86_64-apple-darwin
TARGET_MACOS_ARM64    := aarch64-apple-darwin
TARGET_WINDOWS_X86_64 := x86_64-pc-windows-msvc
TARGET_WINDOWS_ARM64  := aarch64-pc-windows-msvc

.PHONY: all build dev run fmt lint check test test-xvfb clean \
        build-linux-x86_64 build-linux-arm64 build-all-linux \
        build-macos-x86_64 build-macos-arm64 build-macos-universal \
        build-windows-x86_64 build-windows-arm64 \
        install uninstall package-linux package-macos package-windows help

all: build

help:
	@echo "qeyes ($(VERSION)) Makefile targets:"
	@echo "  make build                 Build native release binary"
	@echo "  make dev                   Run qeyes in debug mode (pass ARGS='...')"
	@echo "  make fmt                   Format Rust code with rustfmt"
	@echo "  make lint                  Run rustfmt check and clippy (-D warnings)"
	@echo "  make test                  Run unit, golden-render, and CLI parity tests"
	@echo "  make test-xvfb             Run full test suite under Xvfb (Linux X11)"
	@echo "  make build-linux-x86_64    Build Linux x86_64 release binary"
	@echo "  make build-linux-arm64     Build Linux arm64 (aarch64) release binary"
	@echo "  make build-macos-x86_64    Build macOS Intel (x86_64) release binary"
	@echo "  make build-macos-arm64     Build macOS Apple Silicon (arm64) release binary"
	@echo "  make build-macos-universal Build macOS Universal binary and QEyes.app bundle"
	@echo "  make build-windows-x86_64  Build Windows x86_64 release binary"
	@echo "  make build-windows-arm64   Build Windows arm64 release binary"
	@echo "  make package-linux         Package Linux .tar.gz archive (TARGET=...)"
	@echo "  make package-macos         Package macOS Universal QEyes.app and .dmg"
	@echo "  make package-windows       Package Windows .zip archive (TARGET=...)"
	@echo "  make install               Install qeyes binary, man page, desktop file & icon"
	@echo "  make uninstall             Remove installed qeyes files"
	@echo "  make clean                 Clean cargo build artifacts and dist/"

build:
	$(CARGO) build --release --locked

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

build-macos-universal: build-macos-x86_64 build-macos-arm64
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS"
	@mkdir -p "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources"
	lipo -create \
		"target/$(TARGET_MACOS_X86_64)/release/$(BINARY_NAME)" \
		"target/$(TARGET_MACOS_ARM64)/release/$(BINARY_NAME)" \
		-output "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"
	chmod 755 "$(DIST_DIR)/$(APP_NAME).app/Contents/MacOS/$(BINARY_NAME)"
	cp assets/Info.plist "$(DIST_DIR)/$(APP_NAME).app/Contents/Info.plist"
	cp assets/qeyes.svg "$(DIST_DIR)/$(APP_NAME).app/Contents/Resources/qeyes.svg"

build-windows-x86_64:
	$(CARGO) build --release --locked --target $(TARGET_WINDOWS_X86_64)

build-windows-arm64:
	$(CARGO) build --release --locked --target $(TARGET_WINDOWS_ARM64)

# ---------------------------------------------------------------------------
# Packaging targets
# ---------------------------------------------------------------------------

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
	tar -C "$(DIST_DIR)" -czf "$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.app.tar.gz" "$(APP_NAME).app"
	@if command -v hdiutil >/dev/null 2>&1; then \
		hdiutil create -volname "$(APP_NAME)" \
			-srcfolder "$(DIST_DIR)/$(APP_NAME).app" \
			-ov -format UDZO \
			"$(DIST_DIR)/$(BINARY_NAME)-$(VERSION)-macos-universal.dmg"; \
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
# Install / Uninstall targets (Linux / Unix)
# ---------------------------------------------------------------------------

install: build
	install -d "$(DESTDIR)$(PREFIX)/bin"
	install -m 755 "target/release/$(BINARY_NAME)" "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"
	install -d "$(DESTDIR)$(PREFIX)/share/man/man1"
	install -m 644 "docs/qeyes.1" "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"
	install -d "$(DESTDIR)$(PREFIX)/share/applications"
	install -m 644 "assets/qeyes.desktop" "$(DESTDIR)$(PREFIX)/share/applications/qeyes.desktop"
	install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps"
	install -m 644 "assets/qeyes.svg" "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/qeyes.svg"

uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/$(BINARY_NAME)"
	rm -f "$(DESTDIR)$(PREFIX)/share/man/man1/qeyes.1"
	rm -f "$(DESTDIR)$(PREFIX)/share/applications/qeyes.desktop"
	rm -f "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/qeyes.svg"

clean:
	$(CARGO) clean
	rm -rf "$(DIST_DIR)"
