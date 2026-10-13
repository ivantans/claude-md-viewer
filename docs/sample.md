# Claude MD Viewer

A small, quiet Markdown viewer for macOS. This file is the sample used for the screenshot in the README.

## Why

Markdown is everywhere, but reading it usually means opening an editor. This app only *shows* a file: warm dark surface, serif text, and tables that are easy to scan.

- Opens `.md` files from Finder, the Open panel, or drag and drop
- Reloads automatically when the file changes on disk
- Table of contents, find, zoom, and print

## Shortcuts

| Action | Shortcut | Notes |
|---|---|---|
| Open a file | `⌘O` | Multiple files open in separate windows |
| Table of contents | `⌘⇧O` | Built from heading levels 1–3 |
| Find in document | `⌘F` | `Enter` for the next match |
| Zoom | `⌘+` / `⌘-` | `⌘0` resets |

## Code blocks

Hover a block to reveal the **Copy** button.

```bash
./build.sh
open "Claude MD Viewer.app" README.md
```

> Links to other Markdown files open in a new window; web links open in your browser.
