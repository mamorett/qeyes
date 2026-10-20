# Testing Guide for `qeyes`

## 1. Automated Tests

Run all unit, CLI parity, persistence, and headless golden-render tests:

```bash
make test
```

On Linux with `xvfb-run` and `xdotool` installed, run the full X11 integration suite:

```bash
make test-xvfb
```

## 2. Lint and Formatting Verification

```bash
make lint
```

## 3. Manual Cross-Platform Verification Checklist

- **Linux X11 (KDE Plasma / GNOME on Xorg / i3 / openbox)**:
  - Launch `qeyes` and verify transparent exterior around the two eyes.
  - Move the mouse across the entire desktop and across multiple monitors; confirm both pupils track the cursor continuously.
  - Test without a compositor (or with `qeyes --force-xshape`) to confirm the 1-bit `XShape` bounding mask removes rectangular window corners.
  - Drag the interior to move; drag the 8 px edges or 14 px corners to resize freely; right-click to open the context menu.
- **Linux Wayland (via XWayland)**:
  - Launch `qeyes --log info` under a Wayland session; confirm the informational XWayland notice is logged and the app runs via XWayland.
- **macOS (Intel `x86_64` & Apple Silicon `arm64`)**:
  - Build universal app bundle with `make build-macos-universal`.
  - Verify borderless transparent window without rectangular drop shadow and global cursor tracking via `NSEvent.mouseLocation` (no Accessibility permission prompt).
- **Windows 10 / 11 (`x86_64` & `arm64`)**:
  - Verify transparent borderless window under DWM, multi-monitor HiDPI scaling, and `--click-through` (`WS_EX_TRANSPARENT`).
