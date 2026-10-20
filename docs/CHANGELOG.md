# Changelog

All notable changes to `qeyes` are documented in this file.

## [0.1.0] - 2026-10-20

### Added
- Complete cross-platform reimplementation of classic X11 `xeyes` in Rust (`qeyes`).
- Global mouse pointer tracking on Linux/X11 (`XQueryPointer`), Windows (`GetCursorPos` + `ScreenToClient`), and macOS (`NSEvent::mouseLocation` + `convertPointFromScreen`).
- Full `xeyes` CLI and Xresource (`-xrm` / `RESOURCE_MANAGER` / `~/.Xresources`) parity: `-geometry`, `-display`, `-fg`/`-foreground`, `-bg`/`-background`, `-outline`, `-center`, `-bw`/`-borderwidth`, `-rv`/`+rv`, `-shape`/`+shape`, `-render`/`+render`/`-no-render`, `-backing`, `-name`, `-title`.
- Anti-aliased CPU vector rendering (`tiny-skia`) plus 1-bit `XShape` bounding/input mask support for compositor-less X11 setups.
- Modern features: right-click context menu, 7 built-in themes (`classic`, `dark`, `neon`, `solarized`, `bloodshot`, `cartoon`, `monochrome`), periodic natural blinking (`--blink`), smooth pupil tracking (`--smooth`), always-on-top (`--always-on-top`), click-through mode (`--click-through`), borderless resize grips, and atomic TOML settings persistence with multi-profile support (`--profile`).
- Root `Makefile` and GitHub Actions workflows (`ci.yml` and `release.yml`) building all target platforms: Linux (`x86_64`, `arm64`), macOS (`x86_64`, `arm64`, Universal `.app`/`.dmg`), and Windows (`x86_64`, `arm64`).
