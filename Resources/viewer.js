// Renders the Markdown sent by the app (see Sources/main.swift) into #content.
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

// Called from Swift with { markdown, name, folder, baseURL, keepScroll }.
function render(doc) {
  const scrollY = doc.keepScroll ? window.scrollY : 0;

  // <base> makes relative images and links resolve against the Markdown file's folder.
  let base = document.querySelector('base');
  if (!base) { base = document.createElement('base'); document.head.appendChild(base); }
  base.href = doc.baseURL;

  const markdown = doc.markdown.replace(/^---\n[\s\S]*?\n---\n/, ''); // skip YAML front matter
  const template = document.createElement('template');
  template.innerHTML = marked.parse(markdown, { gfm: true });
  sanitize(template.content);
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

// Anchor links: with <base> set, "#section" would resolve against the folder, so handle them here.
document.addEventListener('click', (event) => {
  const link = event.target.closest('a[href]');
  if (!link) return;
  const href = link.getAttribute('href');
  if (href.startsWith('#')) {
    event.preventDefault();
    const target = document.getElementById(decodeURIComponent(href.slice(1)));
    if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    if (window.innerWidth < 1180) document.body.classList.remove('toc-open');
  }
});

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
  if (!window.find(text, false, !!backwards, true)) { findInput.focus(); findInput.select(); }
}
document.addEventListener('keydown', (event) => {
  if (findBar.hidden) return;
  if (event.key === 'Enter') { event.preventDefault(); findNext(event.shiftKey); }
  else if (event.key === 'Escape') closeFind();
});
document.getElementById('find-close').addEventListener('click', closeFind);
