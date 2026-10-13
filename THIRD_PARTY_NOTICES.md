# Third-party notices

Claude MD Viewer bundles the following software.

## Rust crates (the application shell)

The executable is built with [Tauri](https://github.com/tauri-apps/tauri) and statically links the Rust crates pinned in `src-tauri/Cargo.lock`. The ones this project depends on directly:

| Crate | License |
|---|---|
| [tauri](https://github.com/tauri-apps/tauri), with [wry](https://github.com/tauri-apps/wry) and [muda](https://github.com/tauri-apps/muda) | Apache-2.0 OR MIT |
| [tao](https://github.com/tauri-apps/tao) | Apache-2.0 |
| [tauri-plugin-dialog](https://github.com/tauri-apps/plugins-workspace), with [rfd](https://github.com/PolyMeilex/rfd) (MIT) | Apache-2.0 OR MIT |
| [tauri-plugin-single-instance](https://github.com/tauri-apps/plugins-workspace) | Apache-2.0 OR MIT |
| [serde_json](https://github.com/serde-rs/json) | MIT OR Apache-2.0 |
| [open](https://github.com/Byron/open-rs) | MIT |

Their transitive dependencies (about 220 crates on macOS and Windows, about 290 on Linux) are almost all under MIT, Apache-2.0, or both. The exceptions are:

- **MPL-2.0**: [cssparser](https://github.com/servo/rust-cssparser), cssparser-macros, [selectors](https://github.com/servo/stylo), [dtoa-short](https://github.com/upsuper/dtoa-short) and [option-ext](https://github.com/soc/option-ext). They are used unmodified; their source is available at the linked repositories and on crates.io.
- **BSD-3-Clause**: [brotli](https://github.com/dropbox/rust-brotli) and brotli-decompressor (also MIT), alloc-no-stdlib and alloc-stdlib.
- **Unicode-3.0**: the [ICU4X](https://github.com/unicode-org/icu4x) crates (`icu_*`, `zerovec`, `yoke`, `litemap`, `tinystr`, `writeable` and related).
- **Zlib**: [foldhash](https://github.com/orlp/foldhash).

To print every crate with its license for your platform:

```bash
cd src-tauri && cargo tree -e normal --prefix none --format "{p} {l}" | sort -u
```

The web view itself is not bundled: the app uses WKWebView on macOS, Microsoft Edge WebView2 on Windows, and WebKitGTK on Linux, as installed on the system.

## marked 12.0.2

https://github.com/markedjs/marked — bundled as `Resources/marked.min.js`. Its license file is reproduced below.

---

# License information

## Contribution License Agreement

If you contribute code to this project, you are implicitly allowing your code
to be distributed under the MIT license. You are also implicitly verifying that
all code is your original work. `</legalese>`

## Marked

Copyright (c) 2018+, MarkedJS (https://github.com/markedjs/)
Copyright (c) 2011-2018, Christopher Jeffrey (https://github.com/chjj/)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

## Markdown

Copyright © 2004, John Gruber
http://daringfireball.net/
All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
* Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
* Neither the name “Markdown” nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

This software is provided by the copyright holders and contributors “as is” and any express or implied warranties, including, but not limited to, the implied warranties of merchantability and fitness for a particular purpose are disclaimed. In no event shall the copyright owner or contributors be liable for any direct, indirect, incidental, special, exemplary, or consequential damages (including, but not limited to, procurement of substitute goods or services; loss of use, data, or profits; or business interruption) however caused and on any theory of liability, whether in contract, strict liability, or tort (including negligence or otherwise) arising in any way out of the use of this software, even if advised of the possibility of such damage.
