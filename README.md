# Claude MD Viewer

A small Markdown viewer for macOS with a reading experience modelled on the Markdown preview in the Claude app: warm dark surface, serif text, and rounded tables that are easy to scan.

> **Unofficial project.** Claude MD Viewer is a community-made tool. It is not affiliated with, sponsored by, or endorsed by Anthropic. "Claude" is a trademark of Anthropic, used here only to describe the look this viewer is inspired by.

![Claude MD Viewer showing a Markdown document with a table and a code block](docs/screenshot.png)

## Features

- Opens `.md` files from Finder (Open With), the Open panel, or drag and drop
- Reloads automatically when the file changes on disk, keeping your scroll position
- Table of contents built from the document's headings
- Copy button on every code block
- Find in document, zoom, and print (or save as PDF)
- Dark, Light, and Match System themes
- Viewer only: it never modifies your files
- Works offline; the Markdown parser is bundled

## Install

There is no prebuilt download yet, so you build it yourself. It takes a few seconds.

Requirements: macOS 13 or later and the Xcode command line tools (`xcode-select --install`).

```bash
git clone https://github.com/ivantans/claude-md-viewer.git
cd claude-md-viewer
./build.sh
```

This produces `Claude MD Viewer.app` in the project folder. Move it to your Applications folder:

```bash
mv "Claude MD Viewer.app" /Applications/
```

Because you built the app locally, macOS opens it without a Gatekeeper warning.

### Make it the default app for Markdown files

1. In Finder, select any `.md` file and press `⌘I` (Get Info).
2. Under **Open with**, choose **Claude MD Viewer**.
3. Click **Change All…**.

## Keyboard shortcuts

| Action | Shortcut |
|---|---|
| Open a file | `⌘O` |
| Reload | `⌘R` |
| Table of contents | `⌘⇧O` |
| Find / Find next | `⌘F` / `⌘G` (or `Enter`) |
| Zoom in / out / reset | `⌘+` / `⌘-` / `⌘0` |
| Print or save as PDF | `⌘P` |

## How it works

The app is a single Swift file using AppKit and `WKWebView`. Each window loads `Resources/viewer.html`; the Swift side reads the Markdown file and passes its text to a JavaScript `render()` function, which parses it with [marked](https://github.com/markedjs/marked), sanitizes the result, and builds the table of contents. A one-second timer checks the file's modification date and re-renders when it changes.

```
Sources/main.swift      App, windows, menus, file watching
Resources/viewer.html   Page shell
Resources/viewer.css    All colors and typography
Resources/viewer.js     Rendering, table of contents, find, copy buttons
Resources/marked.min.js Bundled Markdown parser (marked 12.0.2)
build.sh                Builds the .app bundle with swiftc
```

### Changing the look

Every color and font is a CSS variable at the top of `Resources/viewer.css`, with one block for dark and one for light. Edit, run `./build.sh`, and reopen the app.

## Known limitations

- No app icon yet
- No syntax highlighting in code blocks
- No signed or notarized release build
- The palette was matched by eye from the Claude app, so it is close but not identical
- Raw HTML in Markdown is rendered after stripping scripts and event handlers; treat files from untrusted sources with the usual care

## Contributing

Issues and pull requests are welcome. The codebase is intentionally small, with no package manager and no build system beyond `build.sh`. Please keep changes in that spirit.

## License

[MIT](LICENSE). Bundled third-party software is listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
