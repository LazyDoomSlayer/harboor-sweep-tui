# Harbor Sweep TUI

[![harboor-sweep](https://snapcraft.io/harboor-sweep/badge.svg)](https://snapcraft.io/harboor-sweep)
![Crates.io Version](https://img.shields.io/crates/v/harboor-sweep?style=flat&link=https%3A%2F%2Fcrates.io%2Fcrates%2Fharboor-sweep)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cross-platform TUI (terminal user interface) tool to identify and manage active ports and their processes.

---

## Features

* **Real-time Port Monitoring**: Automatically refreshes the list of open ports and their associated processes.
* **Search & Filter**: Instantly filter by PID, port number, or process name using the built-in search bar.
* **Sortable Columns**: Press number keys (`1`–`4`) to sort by Port, PID, Name, or Path, and toggle sort direction with
  a keypress.
* **Interactive TUI**: Keyboard-driven interface with Vim-style navigation.
* **Kill Processes**: Safely terminate processes holding unwanted ports.
* **Color Themes**: Switch between multiple Tailwind-inspired palettes.
* **Help Popup**: On-demand keybindings reference.

## 🔧 Once Started

### 🧭 **Navigation**

* `j` / `Down Arrow`: Move selection down
* `k` / `Up Arrow`: Move selection up
* `PageUp` / `PageDown`: Scroll one page
* `Shift+PageUp` / `Shift+PageDown`: Jump to first or last row

### 🔍 **Search**

* `Ctrl+F`: Toggle search bar
* `e`: Enter editing mode (focus search field)
* Type: Filter by PID, port, or process name
* `Backspace`: Delete from search
* `Left` / `Right`: Move cursor in input
* `Enter` / `Up` / `Down`: Submit search + move selection
* `Esc`: Exit search editing

### 🧨 **Kill Process**

* `k`: Open kill-process confirmation for selected row
* `←` / `→`: Select “Kill” or “Cancel”
* `Enter`: Confirm kill or cancel
* `Esc`: Cancel/abort

### 🧰 **Sorting**

* `1`: Sort by Port (press again to toggle ▲/▼)
* `2`: Sort by PID (press again to toggle ▲/▼)
* `3`: Sort by Process Name (press again to toggle ▲/▼)
* `4`: Sort by Process Path (press again to toggle ▲/▼)

### 🎨 **Themes**

* `Shift+Right` / `l`: Cycle to next color theme
* `Shift+Left` / `h`: Cycle to previous color theme

### ❓ **Help**

* `F1` or `?`: Toggle keybindings popup
* `Up` / `Down`: Navigate help
* `PageUp` / `PageDown`: Page through help
* `Shift+PageUp` / `Shift+PageDown`: Jump to top/bottom of help
* `Esc`, `F1`, `?`: Exit help view

### 🚪 **Exit**

* `q`, `Esc`, or `Ctrl+C`: Quit the application

## Configuration

No external configuration files are required—everything runs out of the box.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
