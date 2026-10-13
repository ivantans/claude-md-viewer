// Claude MD Viewer: a small Markdown viewer for macOS.
// Each window hosts a WKWebView that loads Resources/viewer.html. The file's contents are passed
// to the JavaScript render() function and reloaded automatically when the file changes on disk.
import AppKit
import UniformTypeIdentifiers
import WebKit

let markdownExtensions: Set<String> = ["md", "markdown", "mdown", "mkd", "txt"]

func windowBackgroundColor() -> NSColor {
    NSColor(name: nil) { appearance in
        appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            ? NSColor(srgbRed: 0x1c / 255, green: 0x1b / 255, blue: 0x19 / 255, alpha: 1)
            : NSColor(srgbRed: 0xfa / 255, green: 0xf9 / 255, blue: 0xf5 / 255, alpha: 1)
    }
}

/// A WKWebView that accepts Markdown files dropped onto the window.
final class DropWebView: WKWebView {
    var onFileDropped: ((URL) -> Void)?

    private func markdownFile(in info: NSDraggingInfo) -> URL? {
        let options: [NSPasteboard.ReadingOptionKey: Any] = [.urlReadingFileURLsOnly: true]
        let urls = info.draggingPasteboard.readObjects(forClasses: [NSURL.self], options: options) as? [URL]
        return urls?.first { markdownExtensions.contains($0.pathExtension.lowercased()) }
    }

    override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
        markdownFile(in: sender) != nil ? .copy : []
    }

    override func draggingUpdated(_ sender: NSDraggingInfo) -> NSDragOperation {
        markdownFile(in: sender) != nil ? .copy : []
    }

    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        guard let url = markdownFile(in: sender) else { return false }
        onFileDropped?(url)
        return true
    }
}

final class DocumentWindow: NSObject, WKNavigationDelegate, NSWindowDelegate {
    let window: NSWindow
    let webView: DropWebView
    private(set) var fileURL: URL?
    private var pageLoaded = false
    private var lastModified: Date?
    private var watcher: Timer?
    var onClose: ((DocumentWindow) -> Void)?

    init(fileURL: URL?) {
        self.fileURL = fileURL
        webView = DropWebView(frame: .zero, configuration: WKWebViewConfiguration())
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1040, height: 900),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered, defer: false)
        super.init()

        window.isReleasedWhenClosed = false
        window.titlebarAppearsTransparent = true
        window.backgroundColor = windowBackgroundColor()
        window.minSize = NSSize(width: 480, height: 360)
        window.contentView = webView
        window.delegate = self
        window.center()
        window.setFrameAutosaveName("DocumentWindow")

        webView.navigationDelegate = self
        webView.setValue(false, forKey: "drawsBackground")  // avoid a white flash before the page loads
        webView.registerForDraggedTypes([.fileURL])
        webView.onFileDropped = { [weak self] url in self?.show(url) }

        let page = Bundle.main.url(forResource: "viewer", withExtension: "html")!
        // Read access to "/" so images referenced by the Markdown file can load.
        webView.loadFileURL(page, allowingReadAccessTo: URL(fileURLWithPath: "/"))
        updateTitle()

        watcher = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in self?.reloadIfChanged() }
    }

    func show(_ url: URL) {
        fileURL = url
        updateTitle()
        load(keepScroll: false)
    }

    private func updateTitle() {
        window.title = fileURL?.lastPathComponent ?? "Claude MD Viewer"
        window.representedURL = fileURL
    }

    private func modificationDate() -> Date? {
        guard let fileURL else { return nil }
        return (try? FileManager.default.attributesOfItem(atPath: fileURL.path))?[.modificationDate] as? Date
    }

    func load(keepScroll: Bool) {
        guard pageLoaded, let fileURL else { return }
        guard let data = try? Data(contentsOf: fileURL) else { return }
        let text = String(data: data, encoding: .utf8) ?? String(decoding: data, as: UTF8.self)
        lastModified = modificationDate()
        let folder = fileURL.deletingLastPathComponent()
        let payload: [String: Any] = [
            "markdown": text,
            "name": fileURL.lastPathComponent,
            "folder": folder.lastPathComponent,
            "baseURL": folder.absoluteString,
            "keepScroll": keepScroll,
        ]
        guard let json = try? JSONSerialization.data(withJSONObject: payload),
              let argument = String(data: json, encoding: .utf8) else { return }
        webView.evaluateJavaScript("render(\(argument))")
    }

    private func reloadIfChanged() {
        guard pageLoaded, let current = modificationDate(), current != lastModified else { return }
        load(keepScroll: true)
    }

    func run(_ script: String) { webView.evaluateJavaScript(script) }

    // MARK: WKNavigationDelegate

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        pageLoaded = true
        load(keepScroll: false)
    }

    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                 decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        guard action.navigationType == .linkActivated, let url = action.request.url else {
            decisionHandler(.allow)
            return
        }
        // Links never replace the viewer page: other Markdown files open in a new window,
        // everything else is handed to the default app.
        if url.isFileURL, markdownExtensions.contains(url.pathExtension.lowercased()) {
            (NSApp.delegate as? AppDelegate)?.open(url)
        } else {
            NSWorkspace.shared.open(url)
        }
        decisionHandler(.cancel)
    }

    // MARK: NSWindowDelegate

    func windowWillClose(_ notification: Notification) {
        watcher?.invalidate()
        onClose?(self)
    }
}

enum Theme: String, CaseIterable {
    case dark, light, system

    var title: String {
        switch self {
        case .dark: return "Dark"
        case .light: return "Light"
        case .system: return "Match System"
        }
    }

    var appearance: NSAppearance? {
        switch self {
        case .dark: return NSAppearance(named: .darkAqua)
        case .light: return NSAppearance(named: .aqua)
        case .system: return nil
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuItemValidation {
    private var documents: [DocumentWindow] = []

    private var activeDocument: DocumentWindow? {
        documents.first { $0.window == NSApp.keyWindow } ?? documents.last
    }

    // Dark by default, like the Markdown preview in the Claude app.
    private var theme: Theme {
        get { Theme(rawValue: UserDefaults.standard.string(forKey: "theme") ?? "") ?? .dark }
        set {
            UserDefaults.standard.set(newValue.rawValue, forKey: "theme")
            NSApp.appearance = newValue.appearance
        }
    }

    func open(_ url: URL?) {
        // An existing empty window is reused for the first file.
        if let url, let empty = documents.first(where: { $0.fileURL == nil }) {
            empty.show(url)
            empty.window.makeKeyAndOrderFront(nil)
            return
        }
        if let url, let existing = documents.first(where: { $0.fileURL == url }) {
            existing.window.makeKeyAndOrderFront(nil)
            return
        }
        let document = DocumentWindow(fileURL: url)
        document.onClose = { [weak self] closed in self?.documents.removeAll { $0 === closed } }
        if let front = documents.last?.window {
            document.window.setFrameTopLeftPoint(front.cascadeTopLeft(from: .zero))
        }
        documents.append(document)
        document.window.makeKeyAndOrderFront(nil)
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.appearance = theme.appearance
        NSApp.mainMenu = buildMenu()
        // application(_:open:) may arrive first; only create an empty window when no file was opened.
        DispatchQueue.main.async { [self] in
            if documents.isEmpty { open(nil) }
            NSApp.activate(ignoringOtherApps: true)
        }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        urls.forEach { open($0) }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    // MARK: Menu actions

    @objc func chooseFile(_ sender: Any?) {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = markdownExtensions.compactMap { UTType(filenameExtension: $0) }
        if panel.runModal() == .OK { panel.urls.forEach { open($0) } }
    }

    @objc func reload(_ sender: Any?) { activeDocument?.load(keepScroll: true) }
    @objc func find(_ sender: Any?) { activeDocument?.run("openFind()") }
    @objc func findNext(_ sender: Any?) { activeDocument?.run("findNext(false)") }
    @objc func toggleTableOfContents(_ sender: Any?) { activeDocument?.run("toggleTOC()") }
    @objc func zoomIn(_ sender: Any?) { if let w = activeDocument?.webView { w.pageZoom = min(w.pageZoom + 0.1, 2.5) } }
    @objc func zoomOut(_ sender: Any?) { if let w = activeDocument?.webView { w.pageZoom = max(w.pageZoom - 0.1, 0.6) } }
    @objc func actualSize(_ sender: Any?) { activeDocument?.webView.pageZoom = 1 }

    @objc func printDocument(_ sender: Any?) {
        guard let document = activeDocument else { return }
        let operation = document.webView.printOperation(with: NSPrintInfo.shared)
        operation.view?.frame = document.webView.bounds
        operation.runModal(for: document.window, delegate: nil, didRun: nil, contextInfo: nil)
    }

    @objc func selectTheme(_ sender: NSMenuItem) {
        if let raw = sender.representedObject as? String, let selected = Theme(rawValue: raw) { theme = selected }
    }

    @objc func showAbout(_ sender: Any?) {
        NSApp.orderFrontStandardAboutPanel(options: [
            .credits: NSAttributedString(
                string: "An unofficial, community-made Markdown viewer.\nNot affiliated with or endorsed by Anthropic.",
                attributes: [.font: NSFont.systemFont(ofSize: 11), .foregroundColor: NSColor.secondaryLabelColor]),
        ])
    }

    func validateMenuItem(_ item: NSMenuItem) -> Bool {
        if item.action == #selector(selectTheme(_:)) {
            item.state = (item.representedObject as? String) == theme.rawValue ? .on : .off
        }
        return true
    }

    private func buildMenu() -> NSMenu {
        func item(_ title: String, _ action: Selector?, _ key: String = "", _ modifiers: NSEvent.ModifierFlags = .command) -> NSMenuItem {
            let menuItem = NSMenuItem(title: title, action: action, keyEquivalent: key)
            menuItem.keyEquivalentModifierMask = modifiers
            return menuItem
        }
        func menu(_ title: String, _ items: [NSMenuItem]) -> NSMenuItem {
            let parent = NSMenuItem()
            let submenu = NSMenu(title: title)
            items.forEach { submenu.addItem($0) }
            parent.submenu = submenu
            return parent
        }

        let main = NSMenu()
        main.addItem(menu("Claude MD Viewer", [
            item("About Claude MD Viewer", #selector(showAbout(_:))),
            .separator(),
            item("Hide Claude MD Viewer", #selector(NSApplication.hide(_:)), "h"),
            item("Quit Claude MD Viewer", #selector(NSApplication.terminate(_:)), "q"),
        ]))
        main.addItem(menu("File", [
            item("Open…", #selector(chooseFile(_:)), "o"),
            item("Reload", #selector(reload(_:)), "r"),
            .separator(),
            item("Print…", #selector(printDocument(_:)), "p"),
            item("Close Window", #selector(NSWindow.performClose(_:)), "w"),
        ]))
        main.addItem(menu("Edit", [
            item("Copy", #selector(NSText.copy(_:)), "c"),
            item("Select All", #selector(NSText.selectAll(_:)), "a"),
            .separator(),
            item("Find…", #selector(find(_:)), "f"),
            item("Find Next", #selector(findNext(_:)), "g"),
        ]))
        let themeItems = Theme.allCases.map { theme -> NSMenuItem in
            let menuItem = item(theme.title, #selector(selectTheme(_:)))
            menuItem.representedObject = theme.rawValue
            return menuItem
        }
        main.addItem(menu("View", [
            item("Table of Contents", #selector(toggleTableOfContents(_:)), "o", [.command, .shift]),
            .separator(),
        ] + themeItems + [
            .separator(),
            item("Zoom In", #selector(zoomIn(_:)), "+"),
            item("Zoom Out", #selector(zoomOut(_:)), "-"),
            item("Actual Size", #selector(actualSize(_:)), "0"),
        ]))
        let windowMenu = menu("Window", [
            item("Minimize", #selector(NSWindow.performMiniaturize(_:)), "m"),
            item("Zoom", #selector(NSWindow.performZoom(_:))),
        ])
        main.addItem(windowMenu)
        NSApp.windowsMenu = windowMenu.submenu
        return main
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
