// Renders the Markdown sent by the app shell (see src-tauri/src/main.rs) into #content.
// The shell calls render(), openFind(), findNext() and toggleTOC(); this page calls back
// through Tauri's invoke() for the two things it cannot do itself (see the end of the file).
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : () => Promise.resolve();
const content = document.getElementById('content');
const toc = document.getElementById('toc');
const breadcrumb = document.getElementById('breadcrumb');
const findBar = document.getElementById('find');
const findInput = document.getElementById('find-input');

// Markdown files can come from anywhere: drop elements and attributes that could run script.
function sanitize(root) {
  root.querySelectorAll('script, iframe, object, embed, link, meta, style, form, base').forEach((el) => el.remove());
  root.querySelectorAll('*').forEach((el) => {
    for (const attr of [...el.attributes]) {
      const value = attr.value.trim().toLowerCase();
      if (attr.name.startsWith('on') || ((attr.name === 'href' || attr.name === 'src') && value.startsWith('javascript:'))) {
        el.removeAttribute(attr.name);
      }
    }
  });
}

function slugify(text, used) {
  let slug = text.toLowerCase().trim().replace(/[^\p{L}\p{N}\s-]/gu, '').replace(/\s+/g, '-') || 'section';
  const base = slug;
  for (let i = 1; used.has(slug); i++) slug = `${base}-${i}`;
  used.add(slug);
  return slug;
}

// Called by the shell with { markdown, name, folder, baseURL, keepScroll }.
function render(doc) {
  const scrollY = doc.keepScroll ? window.scrollY : 0;

  // <base> makes relative images and links resolve against the Markdown file's folder.
  let base = document.querySelector('base');
  if (!base) { base = document.createElement('base'); document.head.appendChild(base); }
  base.href = doc.baseURL;

  const markdown = doc.markdown.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, ''); // skip YAML front matter
  const template = document.createElement('template');
  template.innerHTML = marked.parse(markdown, { gfm: true });
  sanitize(template.content);
  // Absolute file:// URLs are served by the shell from the same origin as the base URL.
  template.content.querySelectorAll('[src^="file://" i]').forEach((el) => {
    el.setAttribute('src', new URL(new URL(el.getAttribute('src')).pathname, doc.baseURL).href);
  });
  content.replaceChildren(template.content);

  content.querySelectorAll('table').forEach((table) => {
    const wrap = document.createElement('div');
    wrap.className = 'table-wrap';
    table.replaceWith(wrap);
    wrap.appendChild(table);
  });

  content.querySelectorAll('pre').forEach((pre) => {
    const code = pre.querySelector('code');
    const language = code && [...code.classList].find((c) => c.startsWith('language-'));
    if (language) {
      const label = document.createElement('span');
      label.className = 'lang';
      label.textContent = language.slice(9);
      pre.appendChild(label);
    }
    const button = document.createElement('button');
    button.className = 'copy';
    button.textContent = 'Copy';
    button.addEventListener('click', () => {
      navigator.clipboard.writeText(code ? code.textContent : pre.textContent).then(() => {
        button.textContent = 'Copied';
        setTimeout(() => { button.textContent = 'Copy'; }, 1200);
      });
    });
    pre.appendChild(button);
  });

  // Table of contents from heading levels 1-3.
  const used = new Set();
  const items = [];
  content.querySelectorAll('h1, h2, h3, h4').forEach((heading) => {
    heading.id = slugify(heading.textContent, used);
    if (heading.tagName === 'H4') return;
    const link = document.createElement('a');
    link.href = '#' + heading.id;
    link.textContent = heading.textContent;
    link.title = heading.textContent;
    link.className = 'l' + heading.tagName[1];
    items.push(link);
  });
  toc.replaceChildren(...items);

  breadcrumb.replaceChildren(document.createTextNode(doc.folder + ' / '), Object.assign(document.createElement('b'), { textContent: doc.name }));
  document.title = doc.name;
  window.scrollTo(0, scrollY);
  markActiveHeading();
}

// Links never replace the viewer page. Anchor links scroll in place (with <base> set, "#section"
// would resolve against the folder); everything else is handed to the shell, which opens other
// Markdown files in a new window and the rest in the default browser or app.
function followLink(event) {
  const link = event.target.closest('a[href]');
  if (!link) return;
  event.preventDefault();
  if (event.type !== 'click') return; // middle click
  const href = link.getAttribute('href');
  if (href.startsWith('#')) {
    let id = href.slice(1);
    try { id = decodeURIComponent(id); } catch (_) {}
    const target = document.getElementById(id);
    if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    if (window.innerWidth < 1180) document.body.classList.remove('toc-open');
  } else if (typeof link.href === 'string') {
    invoke('open_link', { url: link.href });
  }
}
document.addEventListener('click', followLink);
document.addEventListener('auxclick', followLink);

function markActiveHeading() {
  const headings = [...content.querySelectorAll('h1, h2, h3')];
  let active = headings[0];
  for (const heading of headings) { if (heading.getBoundingClientRect().top <= 80) active = heading; else break; }
  toc.querySelectorAll('a').forEach((a) => a.classList.toggle('active', !!active && a.getAttribute('href') === '#' + active.id));
}
let scrollTimer;
window.addEventListener('scroll', () => { clearTimeout(scrollTimer); scrollTimer = setTimeout(markActiveHeading, 60); }, { passive: true });

function toggleTOC() {
  const open = document.body.classList.toggle('toc-open');
  try { localStorage.setItem('toc-open', open ? '1' : '0'); } catch (_) {}
}
document.getElementById('toc-toggle').addEventListener('click', toggleTOC);
try {
  const saved = localStorage.getItem('toc-open');
  if (saved === '1' || (saved === null && window.innerWidth >= 1180)) document.body.classList.add('toc-open');
} catch (_) {}

function openFind() { findBar.hidden = false; findInput.focus(); findInput.select(); }
function closeFind() { findBar.hidden = true; window.getSelection().removeAllRanges(); }

// window.find highlights the next match in the document. Focus leaves the input so the
// highlight is visible; the next Enter (anywhere) continues, Shift+Enter goes backwards.
function findNext(backwards) {
  const text = findInput.value;
  if (!text) return;
  findInput.blur();
  findInput.value = ''; // otherwise window.find matches the query inside the find field itself
  const found = window.find(text, false, !!backwards, true);
  findInput.value = text;
  if (!found) { findInput.focus(); findInput.select(); }
}
document.addEventListener('keydown', (event) => {
  if (findBar.hidden) return;
  if (event.key === 'Enter') { event.preventDefault(); findNext(event.shiftKey); }
  else if (event.key === 'Escape') closeFind();
});
document.getElementById('find-close').addEventListener('click', closeFind);

// Outside macOS the webview sees keys before the menu does, and WebView2 on Windows has its own
// Ctrl+F, Ctrl+G, Ctrl+P and Ctrl+R. Catch the menu's shortcuts here and pass them to the shell.
if (!/Mac/i.test(navigator.platform)) {
  document.getElementById('toc-toggle').title = 'Table of contents (Ctrl+Shift+O)';
  document.getElementById('open-shortcut').textContent = 'Ctrl+O';

  const shortcuts = {
    o: 'open', r: 'reload', p: 'print', w: 'close', q: 'quit', f: 'find', g: 'find-next',
    'shift+o': 'toc', '=': 'zoom-in', '+': 'zoom-in', 'shift+=': 'zoom-in', 'shift++': 'zoom-in', '-': 'zoom-out', 0: 'zoom-reset',
  };
  document.addEventListener('keydown', (event) => {
    if (!event.ctrlKey || event.altKey || event.metaKey) return;
    const id = shortcuts[(event.shiftKey ? 'shift+' : '') + event.key.toLowerCase()];
    if (!id) return;
    event.preventDefault();
    invoke('action', { id });
  });
}

// Tell the shell the page is ready; it answers by calling render() if this window has a file.
invoke('ready');
