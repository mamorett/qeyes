# qeyes

**`qeyes`** is a fast, lightweight, modern cross-platform reimplementation of the classic X11 `xeyes` written in Rust. It displays a pair of resizable eyes on your desktop whose pupils follow your mouse pointer anywhere on the screen across all monitors.

---

## Features

- **True Global Pointer Tracking**:
  - **Linux (X11 / XWayland)**: `XQueryPointer` via pure-Rust `x11rb` (zero special permissions).
  - **macOS (Intel & Apple Silicon)**: `NSEvent.mouseLocation` + window coordinate mapping (no Accessibility permission required).
  - **Windows 10/11 (`x86_64` & `arm64`)**: Win32 `GetCursorPos` + `ScreenToClient` with Per-Monitor HiDPI awareness.
- **100% Classic `xeyes` CLI & XResource Parity**:
  - Supports `-geometry WxH+X+Y` (including negative offsets like `-0-0`), `-fg`, `-bg`, `-outline`, `-center`, `-bw`, `-rv`/`+rv`, `-shape`/`+shape`, `-render`/`+render`/`-no-render`, `-display`, `-name`, `-title`, and `-xrm 'Eyes.foreground: red'`.
  - Embedded X11 `rgb.txt` color table (`SlateBlue`, `light goldenrod yellow`, `grey50`, `#rgb`, `#rrggbb`, `rgb:r/g/b`).
- **Shaped & Per-Pixel Transparent Windows**:
  - Smooth anti-aliased alpha compositing on modern desktops (KWin, Mutter, macOS Aqua, Windows DWM), plus automatic 1-bit **X11 Shape extension (`XShape`)** bounding/input masks on compositor-less X11 window managers.
- **Modern Extras**:
  - **Freely Resizable & Persistent**: Drag the interior to move, drag edges/corners or use `+`/`-`/mouse wheel to resize. Size, position, and settings are atomically saved per user (and per `--profile NAME`).
  - **Interactive Right-Click Context Menu**: Toggle Always on Top, Shaped Window, Antialiasing, Aspect Lock, Blinking, Smooth Tracking, Run at Login, or cycle through 7 built-in themes (*Classic*, *Dark*, *Neon*, *Solarized*, *Bloodshot*, *Cartoon*, *Monochrome*).
  - **Near-Zero Idle CPU**: Adaptive polling drops from 60 Hz during cursor motion to 10 Hz / 4 Hz when idle and only redraws when pupil coordinates change.

---

## Building & Installation

A `Makefile` is included for building, testing, cross-compiling, packaging, and installing across all platforms:

```bash
# Build native release binary
make build

# Run unit, CLI parity, and Xvfb integration tests
make test-xvfb

# Install binary, man page, .desktop entry, and SVG icon (default PREFIX=/usr/local)
sudo make install
```

### Multi-Platform Build Targets

| Platform | Architecture | Makefile Target | Rust Target Triple |
| --- | --- | --- | --- |
| **Linux** | `x86_64` | `make build-linux-x86_64` | `x86_64-unknown-linux-gnu` |
| **Linux** | `arm64` (`aarch64`) | `make build-linux-arm64` | `aarch64-unknown-linux-gnu` |
| **macOS** | `x86_64` (Intel) | `make build-macos-x86_64` | `x86_64-apple-darwin` |
| **macOS** | `arm64` (Apple Silicon) | `make build-macos-arm64` | `aarch64-apple-darwin` |
| **macOS** | Universal (`QEyes.app` & `.dmg`) | `make package-macos` | `x86_64` + `aarch64` (`lipo`) |
| **Windows** | `x86_64` | `make build-windows-x86_64` | `x86_64-pc-windows-msvc` |
| **Windows** | `arm64` | `make build-windows-arm64` | `aarch64-pc-windows-msvc` |

---

## Usage Examples

```bash
# Classic launch
qeyes

# Classic xeyes flags: red pupils/outline, white interior, top-right corner
qeyes -fg red -bg white -geometry 250x140-20+20

# Reverse video with Xresource override
qeyes -rv -xrm 'Eyes.outline: gold'

# Modern neon theme, always on top, natural blinking, smooth pupil motion
qeyes --theme neon --always-on-top --blink --smooth

# Run a second independent pair of eyes with its own saved config profile
qeyes --profile right-monitor --theme cartoon
```

---

## Interactive Controls

| Input | Action |
| --- | --- |
| **Left-drag interior** | Move the window |
| **Left-drag edges / corners** | Resize the window |
| **Right-click** | Open / close the interactive context menu |
| **`+` / `-` or Mouse Wheel** | Enlarge / shrink window by 10% |
| **`0`** | Reset window to default size (`150x100`) |
| **`A`** | Toggle Always on Top |
| **`S`** | Toggle Shaped / Transparent window |
| **`R`** | Toggle Reverse Video |
| **`B`** | Toggle Blinking Eyes |
| **`T`** | Cycle built-in Themes |
| **`Escape` / `Q`** | Save settings and quit |

---

## Wayland Note (Linux)

Wayland's security model intentionally prevents ordinary client applications from polling the global cursor coordinates outside their own surface. On Linux, `qeyes` automatically uses the X11 backend (running under XWayland in Wayland sessions), which tracks the pointer across all X11/XWayland windows and root surfaces. For full desktop tracking on Linux, an X11 session (KDE Plasma X11, GNOME on Xorg, i3, XFCE, etc.) is recommended.

---

## License

Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Mattia Moretti.
