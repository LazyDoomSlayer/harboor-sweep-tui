# harboor-sweep-tui

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cross-platform TUI (terminal user interface) tool to identify and manage active ports and their processes.

---

## Features

* **Real-time Port Monitoring**: Automatically refreshes the list of open ports and their associated processes.
* **Search & Filter**: Instant search across PID, port number, and process name.
* **Interactive TUI**: Keyboard-driven interface with Vim-style navigation.
* **Kill Processes**: Safely terminate processes holding unwanted ports.
* **Color Themes**: Switch between multiple Tailwind-inspired palettes.
* **Help Popup**: On-demand keybindings reference.

Once started:

* **Navigation**

    * `j` / Down Arrow: Move selection down
    * `k` / Up Arrow: Move selection up
    * `PageUp` / `PageDown`: Scroll by page
    * `Shift+PageUp` / `Shift+PageDown`: Jump to first/last row
* **Search**

    * `Ctrl+F`: Toggle search bar
    * Type to filter by PID, port, or process name
    * `Esc`: Exit search
* **Kill Process**

    * `k` on selected row: Open kill confirmation
    * `←`/`→`: Choose between “Kill” or “Cancel”
    * `Enter`: Confirm
* **Help**

    * `F1` or `?`: Toggle help popup
* **Themes**

    * `Shift+Right` / `l`: Next color theme
    * `Shift+Left`  / `h`: Previous color theme
* **Exit**

    * `q`, `Esc`, or `Ctrl+C`

## Configuration

No external configuration files are required—everything runs out of the box.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
