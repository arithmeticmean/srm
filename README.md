# srm - Safe Remove CLI

`srm` is a safer alternative to the classic Unix `rm` command. Instead of permanently deleting files, `srm` moves them to the system Trash, following the [FreeDesktop.org Trash Specification](https://specifications.freedesktop.org/trash-spec/trashspec-latest.html).

This allows users to recover accidentally "removed" files, making `srm` a more forgiving and user-friendly replacement for `rm`.

---

## 🚀 Features

- Mimics `rm` command behavior for easier adoption.
- Supports commonly used flags:  
  - `-r` : Recursively remove directories  
  - `-d` : Remove empty directories
- Complies with the [FreeDesktop Trash Specification](https://specifications.freedesktop.org/trash-spec/trashspec-latest.html)
- **Files are not deleted**, just moved to the system Trash.

> ✅ Future goal: Full compatibility with GNU `rm`, including all its flags and behavior nuances.

---

## 📦 Installation

> Coming soon: Pre-built binaries and package manager instructions.

For now, clone and build from source:

```bash
cargo install -git https://github.com/yourusername/safe-remove.git
```

## 📂 Trash Specification

`srm` follows the FreeDesktop Trash Specification, ensuring compatibility with desktop environments and standard trash-handling tools.

Deleted files are stored in:

    ~/.local/share/Trash/files/ — for file data

    ~/.local/share/Trash/info/ — for metadata (original path, deletion time, etc.)
---

## 📅 Roadmap

- Basic flag support (-r, -d)

- Full GNU `rm` compatibility

- Cross-platform support (macOS, Windows WSL)

- Restore functionality (srm-restore)

- Configurable trash directory

---

## ⚠️ Disclaimer

`srm` is still in early development. Always double-check before removing important files. While it uses Trash instead of deletion, bugs can happen!

---

## 📄 License

MIT License. See LICENSE for details.

---
