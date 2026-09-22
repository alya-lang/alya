use super::extractor::DocModule;

// Official Alya 3D Delta Prism docs mark. Single source of truth lives in
// assets/brand/docs/alya-docs.svg (embedded at compile time).
const ALYA_LOGO_SVG: &str = include_str!("../../../assets/brand/docs/alya-docs.svg");

// Lucide-style sun/moon glyphs for the docs theme toggle.
const SUN_SVG: &str = r##"<svg class="icon-sun" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>"##;
const MOON_SVG: &str = r##"<svg class="icon-moon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>"##;

// Lucide-style hamburger glyphs for the mobile drawer toggles.
const MENU_SVG: &str = r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="18" y2="18"/></svg>"##;
const LIST_SVG: &str = r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="8" x2="21" y1="6" y2="6"/><line x1="8" x2="21" y1="12" y2="12"/><line x1="8" x2="21" y1="18" y2="18"/><line x1="3" x2="3.01" y1="6" y2="6"/><line x1="3" x2="3.01" y1="12" y2="12"/><line x1="3" x2="3.01" y1="18" y2="18"/></svg>"##;

// Official GitHub vector icon from svgl.app
const GITHUB_LOGO_SVG: &str = r##"<svg class="github-icon" width="20" height="20" viewBox="0 0 1024 1024" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path fill-rule="evenodd" clip-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8C0 11.54 2.29 14.53 5.47 15.59C5.87 15.66 6.02 15.42 6.02 15.21C6.02 15.02 6.01 14.39 6.01 13.72C4 14.09 3.48 13.23 3.32 12.78C3.23 12.55 2.84 11.84 2.5 11.65C2.22 11.5 1.82 11.13 2.49 11.12C3.12 11.11 3.57 11.7 3.72 11.94C4.44 13.15 5.59 12.81 6.05 12.6C6.12 12.08 6.33 11.73 6.56 11.53C4.78 11.33 2.92 10.64 2.92 7.58C2.92 6.71 3.23 5.99 3.74 5.43C3.66 5.23 3.38 4.41 3.82 3.31C3.82 3.31 4.49 3.1 6.02 4.13C6.66 3.95 7.34 3.86 8.02 3.86C8.7 3.86 9.38 3.95 10.02 4.13C11.55 3.09 12.22 3.31 12.22 3.31C12.66 4.41 12.38 5.23 12.3 5.43C12.81 5.99 13.12 6.7 13.12 7.58C13.12 10.65 11.25 11.33 9.47 11.53C9.76 11.78 10.01 12.26 10.01 13.01C10.01 14.08 10 14.94 10 15.21C10 15.42 10.15 15.67 10.55 15.59C13.71 14.53 16 11.53 16 8C16 3.58 12.42 0 8 0Z" transform="scale(64)" fill="currentColor"/>
</svg>"##;

const COMMON_CSS: &str = r##"
/* Alya Docs stylesheet (shadcn-inspired). Tokens first, components after.
   Theme: [data-theme="light"] default, [data-theme="dark"] override. */
:root,
[data-theme="light"] {
  --bg:        #ffffff;
  --bg-soft:   #fafafa;
  --muted:     #f4f4f5;
  --fg:        #09090b;
  --fg-muted:  #71717a;
  --fg-faint:  #a1a1aa;
  --border:    #e4e4e7;
  --code-bg:   #f4f4f5;
  --code-fg:   #09090b;
  --accent:    #7c3aed;
  --accent-2:  #0284c7;
  --grad:      linear-gradient(135deg, #7c3aed, #0284c7);
  --good:      #15803d;
  --shadow:    0 1px 2px rgba(0, 0, 0, 0.05);
  color-scheme: light;
}
[data-theme="dark"] {
  --bg:        #09090b;
  --bg-soft:   #101014;
  --muted:     #27272a;
  --fg:        #fafafa;
  --fg-muted:  #a1a1aa;
  --fg-faint:  #63636b;
  --border:    #27272a;
  --code-bg:   #18181b;
  --code-fg:   #f4f4f5;
  --accent:    #c084fc;
  --accent-2:  #38bdf8;
  --good:      #4ade80;
  --shadow:    0 1px 2px rgba(0, 0, 0, 0.4);
  color-scheme: dark;
}
:root {
  --font-sans: "Inter", "Segoe UI", system-ui, -apple-system, sans-serif;
  --font-mono: "JetBrains Mono", "Cascadia Code", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  --radius: 0.5rem;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font-family: var(--font-sans);
  font-size: 14.5px;
  line-height: 1.7;
  -webkit-font-smoothing: antialiased;
}
a { color: inherit; }
::selection { background: rgba(124, 58, 237, 0.22); }

/* Topbar */
.topbar {
  position: sticky;
  top: 0;
  z-index: 50;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}
.topbar-in {
  max-width: 1400px;
  margin: 0 auto;
  display: flex;
  align-items: center;
  gap: 12px;
  height: 57px;
  padding: 0 24px;
}
.brand { display: flex; align-items: center; gap: 10px; text-decoration: none; }
.brand .logo, .brand .alya-logo { width: 22px; height: 22px; flex: none; }
.brand b { font-size: 0.95rem; font-weight: 700; letter-spacing: -0.01em; }
.brand span { color: var(--fg-muted); font-size: 0.85rem; }
.cmdk {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  width: 240px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 5px 8px 5px 10px;
  color: var(--fg-faint);
  font-size: 0.82rem;
}
.cmdk input {
  background: transparent;
  border: 0;
  outline: 0;
  color: var(--fg);
  font-size: 0.82rem;
  width: 100%;
}
.cmdk input::placeholder { color: var(--fg-faint); }
.cmdk kbd {
  font-family: var(--font-sans);
  font-size: 0.7rem;
  background: var(--muted);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 5px;
  white-space: nowrap;
}
.iconbtn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--border);
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  color: var(--fg-muted);
  font-size: 0.95rem;
}
.iconbtn:hover { background: var(--muted); color: var(--fg); }
.iconbtn svg { width: 16px; height: 16px; flex: none; }
/* Right-aligned header action group (theme, GitHub, TOC toggle). */
.topbar-right { display: flex; align-items: center; gap: 12px; }
[data-theme="light"] .icon-sun { display: none; }
[data-theme="dark"] .icon-moon { display: none; }

/* Buttons & badges */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8rem;
  font-weight: 500;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--fg);
  border-radius: 6px;
  padding: 5px 12px;
  cursor: pointer;
  text-decoration: none;
}
.btn:hover { background: var(--muted); }
.badge {
  display: inline-flex;
  align-items: center;
  font-size: 0.7rem;
  font-weight: 600;
  border-radius: 999px;
  padding: 2px 10px;
  border: 1px solid transparent;
  white-space: nowrap;
}
.badge-secondary { background: var(--muted); color: var(--fg); border-color: var(--border); }
.badge-outline { background: transparent; color: var(--fg-muted); border-color: var(--border); }
/* Legacy kind badges map onto the badge system (kept for compatibility). */
.badge-pub, .badge-func, .badge-struct, .badge-enum,
.badge-interface, .badge-const, .badge-kind, .badge-amber {
  background: var(--muted);
  color: var(--fg);
  border-color: var(--border);
}

/* Shell: sidebar + content + toc */
.shell {
  max-width: 1400px;
  margin: 0 auto;
  display: grid;
  grid-template-columns: 250px minmax(0, 1fr) 220px;
  align-items: start;
}
.sidenav {
  position: sticky;
  top: 57px;
  max-height: calc(100vh - 57px);
  overflow-y: auto;
  padding: 24px 12px 48px 24px;
  font-size: 0.85rem;
}
.sidenav details { margin-bottom: 16px; }
.sidenav summary {
  cursor: pointer;
  list-style: none;
  font-weight: 600;
  font-size: 0.78rem;
  color: var(--fg);
  padding: 4px 8px;
  display: flex;
  justify-content: space-between;
}
.sidenav summary::-webkit-details-marker { display: none; }
.sidenav summary .n { color: var(--fg-faint); font-weight: 400; }
.sidenav a {
  display: block;
  color: var(--fg-muted);
  text-decoration: none;
  padding: 4px 8px;
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 0.8rem;
}
.sidenav a:hover { background: var(--muted); color: var(--fg); }
.sidenav a.active { background: var(--muted); color: var(--fg); font-weight: 600; }
.toc {
  position: sticky;
  top: 57px;
  max-height: calc(100vh - 57px);
  overflow-y: auto;
  padding: 24px 24px 48px 12px;
  font-size: 0.8rem;
}
.toc h5 { margin: 0 0 8px; font-size: 0.75rem; font-weight: 600; color: var(--fg); }
.toc a { display: block; color: var(--fg-muted); text-decoration: none; padding: 3px 0 3px 12px; border-left: 2px solid var(--border); }
.toc a:hover { color: var(--fg); }
.toc a.on { color: var(--fg); border-left-color: var(--fg); font-weight: 500; }

/* Content */
.content { padding: 28px 40px 96px; min-width: 0; }
.crumbs { display: flex; align-items: center; gap: 8px; font-size: 0.8rem; color: var(--fg-faint); margin-bottom: 12px; }
.crumbs a { color: var(--fg-muted); text-decoration: none; }
.crumbs a:hover { color: var(--fg); }
.crumbs .sep { color: var(--border); }
h1.title { font-size: 1.9rem; letter-spacing: -0.03em; margin: 0 0 8px; font-weight: 800; }
h1.title code { font-family: var(--font-mono); font-weight: 800; }
.lede { color: var(--fg-muted); margin: 0 0 16px; max-width: 70ch; }
.badgerow { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 24px; }

/* Code blocks */
.codeblock {
  position: relative;
  background: var(--code-bg);
  color: var(--code-fg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px 16px;
  font-family: var(--font-mono);
  font-size: 0.82rem;
  overflow-x: auto;
  margin: 0 0 20px;
  white-space: pre;
}
.codeblock pre { margin: 0; font: inherit; }
.codeblock code { font: inherit; background: none; }
.codeblock .copy {
  position: absolute;
  top: 8px;
  right: 8px;
}
.copy {
  font-size: 0.72rem;
  font-weight: 500;
  background: var(--bg);
  color: var(--fg-muted);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 3px 9px;
  cursor: pointer;
}
.copy:hover { color: var(--fg); }
.tok-k, .kw { color: var(--accent); }
.tok-t, .ty { color: var(--accent-2); }
.tok-s { color: var(--good); }
.arr { color: var(--fg-muted); }

/* Function sections */
.fn { margin-bottom: 40px; scroll-margin-top: 76px; }
h2.sect {
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin: 40px 0 16px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
.fn h2 { font-size: 1.3rem; letter-spacing: -0.02em; margin: 0 0 4px; font-weight: 700; }
.fn h2 code { font-family: var(--font-mono); }
.fn h2 a { color: var(--fg); text-decoration: none; }
.fn h2 a:hover { text-decoration: underline; text-decoration-color: var(--border); }
.fn .anchor { color: var(--fg-faint); font-size: 0.9rem; margin-left: 6px; opacity: 0; }
.fn:hover .anchor { opacity: 1; }
.doc { color: var(--fg); }
.doc p { margin: 0 0 10px; }
.doc h4 { font-size: 0.85rem; margin: 18px 0 6px; font-weight: 600; }
.doc ul { margin: 6px 0 10px 20px; padding: 0; }
.doc li { margin: 2px 0; }
.doc code, code.inline {
  font-family: var(--font-mono);
  font-size: 0.8em;
  background: var(--muted);
  border-radius: 4px;
  padding: 1px 5px;
}

/* Section labels emitted by the markdown renderer for raw ### sections. */
.section-label {
  font-size: 0.78rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--fg-muted);
  margin: 16px 0 8px;
}

/* Alert callout */
.alert {
  display: flex;
  gap: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px 14px;
  margin-top: 12px;
  font-size: 0.85rem;
}
.alert .ico { flex: none; }
.alert b { display: block; font-size: 0.8rem; margin-bottom: 2px; }
.alert p { margin: 0; color: var(--fg-muted); }
.alert-ok { border-left: 3px solid var(--good); }

/* Tables */
.tblwrap { border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; margin-top: 8px; }
table.tbl { width: 100%; border-collapse: collapse; font-size: 0.83rem; }
table.tbl th {
  text-align: left;
  font-weight: 500;
  color: var(--fg-muted);
  background: var(--bg-soft);
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  font-size: 0.75rem;
}
table.tbl td { padding: 8px 12px; border-bottom: 1px solid var(--border); vertical-align: top; }
table.tbl tr:last-child td { border-bottom: 0; }
table.tbl td:first-child { font-family: var(--font-mono); white-space: nowrap; }
table.tbl tr:hover td { background: var(--bg-soft); }

/* Index: hero + cards */
.hero { padding: 40px 0 8px; }
.hero h1 { font-size: 2.4rem; letter-spacing: -0.04em; margin: 0 0 10px; font-weight: 800; }
.hero h1 .grad {
  background: var(--grad);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}
.hero p { color: var(--fg-muted); max-width: 66ch; margin: 0; }
.modgroup { margin-top: 32px; scroll-margin-top: 76px; }
.modgroup > h2 { font-size: 0.95rem; font-weight: 600; margin: 0 0 12px; letter-spacing: -0.01em; }
.modgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
.modcard {
  display: block;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  color: var(--fg);
  text-decoration: none;
  background: var(--bg);
  box-shadow: var(--shadow);
}
.modcard:hover { border-color: var(--fg-faint); text-decoration: none; }
.modcard code { font-family: var(--font-mono); font-size: 0.9rem; font-weight: 600; }
.modcard p { font-size: 0.82rem; color: var(--fg-muted); margin: 6px 0 12px; min-height: 2.5em; }
.footer {
  max-width: 1400px;
  margin: 0 auto;
  padding: 20px 24px 40px;
  border-top: 1px solid var(--border);
  color: var(--fg-muted);
  font-size: 0.8rem;
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
}
.footer a { color: var(--fg-muted); }

/* Hamburger buttons: hidden on desktop, shown per breakpoint below. */
.hamb { display: none; }
/* Scrim behind open drawers (mobile only). */
.scrim { display: none; }

/* Responsive: sidebars become hamburger drawers on small screens */
@media (max-width: 1200px) {
  .shell { grid-template-columns: 230px minmax(0, 1fr); }
  #tocToggle { display: inline-flex; }
  .toc {
    position: fixed;
    top: 57px;
    right: 0;
    bottom: 0;
    width: 264px;
    max-height: none;
    background: var(--bg);
    border-left: 1px solid var(--border);
    z-index: 60;
    transform: translateX(100%);
    transition: transform 0.2s ease;
    padding: 20px 16px 48px;
  }
  .toc.open { transform: none; }
}
@media (max-width: 860px) {
  .shell { grid-template-columns: 1fr; }
  .topbar-right { margin-left: auto; }
  #navToggle { display: inline-flex; }
  .sidenav {
    position: fixed;
    top: 57px;
    left: 0;
    bottom: 0;
    width: 268px;
    max-height: none;
    background: var(--bg);
    border-right: 1px solid var(--border);
    z-index: 60;
    transform: translateX(-100%);
    transition: transform 0.2s ease;
    padding: 16px 12px 48px;
  }
  .sidenav.open { transform: none; }
  .scrim.open {
    display: block;
    position: fixed;
    top: 57px;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 55;
  }
  .content { padding: 20px 20px 64px; }
  .cmdk { display: none; }
}
"##;

pub fn generate_html(module: &DocModule) -> String {
    generate_html_with_nav(module, &[])
}

/// Generate a module page, with sibling-module navigation when the caller
/// passes the full module list (directory mode). Single-file mode passes an
/// empty list and gets a back-link plus page-local navigation instead.
pub fn generate_html_with_nav(module: &DocModule, all_modules: &[DocModule]) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"light\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!(
        "<title>Module {} - Alya Docs</title>\n",
        escape_html(&module.name)
    ));
    // Apply a persisted theme before first paint to avoid a flash.
    html.push_str("<script>try{if(localStorage.getItem('alya-docs-theme')==='dark')document.documentElement.dataset.theme='dark';}catch(e){}</script>\n");
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str("</style>\n</head>\n<body>\n");

    // Topbar: nav hamburger, brand, version, search (with ⌘K hint),
    // theme toggle, GitHub, TOC hamburger.
    html.push_str("<header class=\"topbar\">\n  <div class=\"topbar-in\">\n");
    html.push_str(&format!(
        "    <button class=\"iconbtn hamb\" id=\"navToggle\" title=\"Modules\" aria-label=\"Toggle module navigation\" onclick=\"toggleDrawer('nav')\">{}</button>\n",
        MENU_SVG
    ));
    html.push_str("    <a class=\"brand\" href=\"index.html\">\n");
    html.push_str(ALYA_LOGO_SVG);
    html.push_str("      <b>Alya</b><span>Docs</span>\n    </a>\n");
    html.push_str(&format!(
        "    <span class=\"badge badge-outline\">v{}</span>\n",
        env!("CARGO_PKG_VERSION")
    ));
    html.push_str("    <label class=\"cmdk\">\u{2315} <input id=\"q\" type=\"search\" placeholder=\"Filter symbols\u{2026}\" autocomplete=\"off\" oninput=\"filterSymbols()\"><kbd>\u{2318}K</kbd></label>\n");
    html.push_str("    <div class=\"topbar-right\">\n");
    html.push_str("    <button class=\"iconbtn\" id=\"themeBtn\" title=\"Toggle theme\" aria-label=\"Toggle theme\">\n");
    html.push_str(MOON_SVG);
    html.push_str(SUN_SVG);
    html.push_str("    </button>\n");
    html.push_str(&format!(
        "    <a class=\"iconbtn\" href=\"https://github.com/alya-lang/alya\" target=\"_blank\" rel=\"noopener noreferrer\" title=\"GitHub\" aria-label=\"GitHub repository\">{}</a>\n",
        GITHUB_LOGO_SVG
    ));
    html.push_str(&format!(
        "    <button class=\"iconbtn hamb\" id=\"tocToggle\" title=\"On this page\" aria-label=\"Toggle page contents\" onclick=\"toggleDrawer('toc')\">{}</button>\n",
        LIST_SVG
    ));
    html.push_str("    </div>\n");
    html.push_str("  </div>\n</header>\n");
    html.push_str("<div class=\"scrim\" id=\"scrim\" onclick=\"closeDrawers()\"></div>\n\n");

    html.push_str("<div class=\"shell\">\n\n");

    // Sidebar: back-link, sibling modules grouped by category (when known),
    // then page-local navigation across every item kind.
    html.push_str("<aside class=\"sidenav\">\n");
    html.push_str("  <a class=\"btn\" href=\"index.html\" style=\"margin-bottom:12px;\">\u{2190} All modules</a>\n");
    if !all_modules.is_empty() {
        html.push_str(&render_nav_siblings(&module.name, all_modules));
    }
    html.push_str("  <details class=\"side-group\" open>\n    <summary>On This Page</summary>\n");
    html.push_str(&render_nav_page_anchors(module));
    html.push_str("  </details>\n");
    html.push_str("</aside>\n\n");

    // Main content.
    html.push_str("<main class=\"content\">\n");
    html.push_str(&format!(
        "  <nav class=\"crumbs\"><a href=\"index.html\">Docs</a><span class=\"sep\">\u{203A}</span><code>{}</code></nav>\n",
        escape_html(&module.name)
    ));
    let title = module_title(&module.name, &module.file_path);
    html.push_str(&format!(
        "  <h1 class=\"title\"><code>{}</code></h1>\n",
        escape_html(&title)
    ));
    if !module.description.is_empty() {
        html.push_str("  <div class=\"lede\">\n");
        html.push_str(&render_markdown_html(&module.description));
        html.push_str("  </div>\n");
    }
    html.push_str(&render_badgerow(module));
    if let Some(import_line) = module_import_line(module) {
        html.push_str(&format!(
            "  <div class=\"codeblock\">{}<button class=\"copy\">Copy</button></div>\n",
            escape_html(&import_line)
        ));
    }

    render_html_constants(&mut html, module);
    render_html_interfaces(&mut html, module);
    render_html_structs(&mut html, module);
    render_html_enums(&mut html, module);
    render_html_functions(&mut html, module);

    html.push_str("</main>\n\n");

    // Right rail: page table of contents with scroll-spy hooks.
    html.push_str("<aside class=\"toc\" id=\"toc\">\n  <h5>On This Page</h5>\n");
    html.push_str(&render_toc_links(module));
    html.push_str("</aside>\n\n");

    html.push_str("</div>\n\n");

    html.push_str("<footer class=\"footer\">\n");
    if is_stdlib_path(&module.file_path) {
        html.push_str(&format!(
            "  <span>Generated with <code class=\"inline\">alya doc</code> &middot; Alya Standard Library v{}</span>\n",
            env!("CARGO_PKG_VERSION")
        ));
    } else {
        html.push_str(&format!(
            "  <span>Generated with <code class=\"inline\">alya doc</code> &middot; v{}</span>\n",
            env!("CARGO_PKG_VERSION")
        ));
    }
    html.push_str("  <span><a href=\"index.html\">\u{2190} All modules</a></span>\n");
    html.push_str("</footer>\n\n");

    // Client-side scripts: theme, ⌘K focus, live filter, copy, scroll-spy.
    html.push_str(
        r##"<script>
try{document.documentElement.dataset.theme=localStorage.getItem('alya-docs-theme')||'light';}catch(e){}
document.getElementById('themeBtn').addEventListener('click', () => {
  const el = document.documentElement;
  el.dataset.theme = el.dataset.theme === 'dark' ? 'light' : 'dark';
  try{localStorage.setItem('alya-docs-theme', el.dataset.theme);}catch(e){}
});
document.addEventListener('keydown', (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    const q = document.getElementById('q');
    if (q) q.focus();
  }
});
function filterSymbols() {
  const input = document.getElementById('q');
  const q = input ? input.value.toLowerCase().trim() : '';
  document.querySelectorAll('.doc-item').forEach(el => {
    const text = el.textContent.toLowerCase();
    el.style.display = !q || text.includes(q) ? '' : 'none';
  });
  document.querySelectorAll('#toc a').forEach(a => {
    const target = document.querySelector(a.getAttribute('href'));
    if (target) a.style.display = target.style.display;
  });
}
document.querySelectorAll('.copy').forEach((btn) => {
  btn.addEventListener('click', async () => {
    const block = btn.closest('.codeblock');
    const code = block ? block.querySelector('code') : null;
    const text = code ? code.innerText : btn.parentElement.innerText.replace(/Copy$/, '');
    try { await navigator.clipboard.writeText(text); }
    catch {
      const ta = document.createElement('textarea');
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      ta.remove();
    }
    const old = btn.textContent;
    btn.textContent = 'Copied';
    setTimeout(() => (btn.textContent = old), 1200);
  });
});
const tocLinks = [...document.querySelectorAll('#toc a')];
const spy = new IntersectionObserver((entries) => {
  entries.forEach((en) => {
    if (en.isIntersecting) {
      tocLinks.forEach((a) => a.classList.toggle('on', a.getAttribute('href') === '#' + en.target.id));
    }
  });
}, { rootMargin: '-20% 0px -70% 0px' });
tocLinks.forEach((a) => {
  const s = document.querySelector(a.getAttribute('href'));
  if (s) spy.observe(s);
});
function toggleDrawer(which) {
  const left = document.querySelector('.sidenav');
  const right = document.querySelector('.toc');
  const scrim = document.getElementById('scrim');
  const wantLeft = which === 'nav' ? !left.classList.contains('open') : false;
  const wantRight = which === 'toc' && right ? !right.classList.contains('open') : false;
  left.classList.toggle('open', wantLeft);
  if (right) right.classList.toggle('open', wantRight);
  if (scrim) scrim.classList.toggle('open', wantLeft || wantRight);
}
function closeDrawers() {
  document.querySelectorAll('.sidenav.open, .toc.open').forEach((el) => el.classList.remove('open'));
  const scrim = document.getElementById('scrim');
  if (scrim) scrim.classList.remove('open');
}
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeDrawers();
});
document.querySelectorAll('.sidenav a').forEach((a) => {
  a.addEventListener('click', () => closeDrawers());
});
</script>
</body>
</html>
"##,
    );

    html
}

/// Display title: `std/name` for embedded stdlib modules, plain name otherwise.
fn module_title(name: &str, file_path: &str) -> String {
    if is_stdlib_path(file_path) && !name.starts_with("std/") {
        format!("std/{}", name)
    } else {
        name.to_string()
    }
}

/// Canonical import line for embedded stdlib modules; None for packages whose
/// import path cannot be inferred (no guessing in generated output).
fn module_import_line(module: &DocModule) -> Option<String> {
    if !is_stdlib_path(&module.file_path) {
        return None;
    }
    let stem = module.name.rsplit('/').next().unwrap_or(&module.name);
    let stem = stem.strip_prefix("std/").unwrap_or(stem);
    Some(format!("import \"std/{}\"", stem))
}

fn is_stdlib_path(file_path: &str) -> bool {
    use std::path::Path;
    Path::new(file_path)
        .components()
        .any(|c| c.as_os_str() == "stdlib")
}

/// Sidebar sibling navigation grouped by module category.
fn render_nav_siblings(current: &str, all: &[DocModule]) -> String {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<&str, Vec<&DocModule>> = BTreeMap::new();
    for m in all {
        let (cat, _, _) = categorize_module(&m.name);
        groups.entry(cat).or_default().push(m);
    }
    let mut out = String::new();
    for (cat, members) in &groups {
        out.push_str("  <details class=\"side-group\" open>\n");
        out.push_str(&format!(
            "    <summary>{} <span class=\"n\">{}</span></summary>\n",
            escape_html(cat),
            members.len()
        ));
        for m in members {
            let target = format!("{}.html", m.name.replace('/', "_"));
            if m.name == current {
                out.push_str(&format!(
                    "    <a href=\"{}\" class=\"active\"><code>{}</code></a>\n",
                    target,
                    escape_html(&m.name)
                ));
            } else {
                out.push_str(&format!(
                    "    <a href=\"{}\"><code>{}</code></a>\n",
                    target,
                    escape_html(&m.name)
                ));
            }
        }
        out.push_str("  </details>\n");
    }
    out
}

/// Sidebar page-local anchors across every item kind.
fn render_nav_page_anchors(module: &DocModule) -> String {
    let mut out = String::new();
    for c in &module.constants {
        out.push_str(&format!(
            "    <a href=\"#c-{}\"><code>{}</code></a>\n",
            escape_html(&c.name),
            escape_html(&c.name)
        ));
    }
    for iface in &module.interfaces {
        out.push_str(&format!(
            "    <a href=\"#if-{}\"><code>{}</code></a>\n",
            escape_html(&iface.name),
            escape_html(&iface.name)
        ));
    }
    for st in &module.structs {
        out.push_str(&format!(
            "    <a href=\"#st-{}\"><code>{}</code></a>\n",
            escape_html(&st.name),
            escape_html(&st.name)
        ));
    }
    for e in &module.enums {
        out.push_str(&format!(
            "    <a href=\"#en-{}\"><code>{}</code></a>\n",
            escape_html(&e.name),
            escape_html(&e.name)
        ));
    }
    for f in &module.functions {
        out.push_str(&format!(
            "    <a href=\"#fn-{}\"><code>{}</code></a>\n",
            escape_html(&f.name),
            escape_html(&f.name)
        ));
    }
    out
}

/// Right-rail table of contents (flat anchor list for scroll-spy).
fn render_toc_links(module: &DocModule) -> String {
    // Same anchor set as the sidebar; spy logic keys off href targets.
    render_nav_page_anchors(module)
}

/// Count badges row under the module lede.
fn render_badgerow(module: &DocModule) -> String {
    let mut out = String::from("  <div class=\"badgerow\">\n");
    let counts = [
        (module.functions.len(), "functions"),
        (module.structs.len(), "structs"),
        (module.enums.len(), "enums"),
        (module.interfaces.len(), "interfaces"),
        (module.constants.len(), "constants"),
    ];
    for (n, label) in counts {
        if n > 0 {
            out.push_str(&format!(
                "    <span class=\"badge badge-secondary\"><b>{}</b>&nbsp;{}</span>\n",
                n, label
            ));
        }
    }
    if is_stdlib_path(&module.file_path) {
        out.push_str("    <span class=\"badge badge-outline\">zero dependencies</span>\n");
        out.push_str("    <span class=\"badge badge-outline\">embedded in compiler</span>\n");
    }
    out.push_str("  </div>\n");
    out
}

fn render_html_constants(html: &mut String, module: &DocModule) {
    if module.constants.is_empty() {
        return;
    }
    html.push_str("<h2 class=\"sect\">Constants</h2>\n");
    for c in &module.constants {
        html.push_str(&format!(
            "<section class=\"fn doc-item\" id=\"c-{}\">\n",
            escape_html(&c.name)
        ));
        html.push_str(&format!(
            "  <h2><a href=\"#c-{}\"><code>{}</code></a>{}</h2>\n",
            escape_html(&c.name),
            escape_html(&c.name),
            if c.is_pub {
                " <span class=\"badge badge-pub\">pub</span>"
            } else {
                ""
            }
        ));
        let sig = format!("pub const {} = {}", c.name, c.value);
        html.push_str(&render_code_block(&sig));
        if !c.doc.is_empty() {
            html.push_str("  <div class=\"doc\">\n");
            html.push_str(&render_markdown_html(&c.doc));
            html.push_str("  </div>\n");
        }
        html.push_str("</section>\n");
    }
}

fn render_html_interfaces(html: &mut String, module: &DocModule) {
    if module.interfaces.is_empty() {
        return;
    }
    html.push_str("<h2 class=\"sect\">Interfaces</h2>\n");
    for iface in &module.interfaces {
        html.push_str(&format!(
            "<section class=\"fn doc-item\" id=\"if-{}\">\n",
            escape_html(&iface.name)
        ));
        html.push_str(&format!(
            "  <h2><a href=\"#if-{}\"><code>{}</code></a>{} <span class=\"badge badge-outline\">interface</span></h2>\n",
            escape_html(&iface.name),
            escape_html(&iface.name),
            if iface.is_pub {
                " <span class=\"badge badge-pub\">pub</span>"
            } else {
                ""
            }
        ));
        if !iface.doc.is_empty() {
            html.push_str("  <div class=\"doc\">\n");
            html.push_str(&render_markdown_html(&iface.doc));
            html.push_str("  </div>\n");
        }
        if !iface.embedded.is_empty() {
            html.push_str("  <h4>Embedded</h4>\n");
            html.push_str(&format!(
                "  <p><code class=\"inline\">{}</code></p>\n",
                escape_html(&iface.embedded.join(", "))
            ));
        }
        if !iface.methods.is_empty() {
            html.push_str("  <h4>Methods</h4>\n");
            html.push_str("  <div class=\"tblwrap\"><table class=\"tbl\">\n");
            html.push_str("    <tr><th>Method</th><th>Parameters</th><th>Returns</th></tr>\n");
            for m in &iface.methods {
                let param_str: Vec<String> = m
                    .params
                    .iter()
                    .map(|p| {
                        if let Some(ref ty) = p.type_ann {
                            format!("{}: {}", p.name, ty)
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect();
                let ret = m.return_type.as_deref().unwrap_or("void");
                html.push_str(&format!(
                    "    <tr><td><code class=\"inline\">{}</code></td><td>{}</td><td>{}</td></tr>\n",
                    escape_html(&m.name),
                    escape_html(&param_str.join(", ")),
                    escape_html(ret)
                ));
            }
            html.push_str("  </table></div>\n");
        }
        html.push_str("</section>\n");
    }
}

fn render_html_structs(html: &mut String, module: &DocModule) {
    if module.structs.is_empty() {
        return;
    }
    html.push_str("<h2 class=\"sect\">Structs</h2>\n");
    for st in &module.structs {
        html.push_str(&format!(
            "<section class=\"fn doc-item\" id=\"st-{}\">\n",
            escape_html(&st.name)
        ));
        html.push_str(&format!(
            "  <h2><a href=\"#st-{}\"><code>{}</code></a>{} <span class=\"badge badge-outline\">struct</span></h2>\n",
            escape_html(&st.name),
            escape_html(&st.name),
            if st.is_pub {
                " <span class=\"badge badge-pub\">pub</span>"
            } else {
                ""
            }
        ));
        if !st.doc.is_empty() {
            html.push_str("  <div class=\"doc\">\n");
            html.push_str(&render_markdown_html(&st.doc));
            html.push_str("  </div>\n");
        }
        if !st.fields.is_empty() {
            html.push_str("  <h4>Fields</h4>\n");
            html.push_str("  <div class=\"tblwrap\"><table class=\"tbl\">\n");
            html.push_str("    <tr><th>Field</th><th>Type</th><th>Default</th></tr>\n");
            for f in &st.fields {
                let ty = f.type_ann.as_deref().unwrap_or("auto");
                let def = f.default_val.as_deref().unwrap_or("-");
                html.push_str(&format!(
                    "    <tr><td><code class=\"inline\">{}</code></td><td><code class=\"inline\">{}</code></td><td>{}</td></tr>\n",
                    escape_html(&f.name),
                    escape_html(ty),
                    escape_html(def)
                ));
            }
            html.push_str("  </table></div>\n");
        }
        if !st.methods.is_empty() {
            html.push_str("  <h4>Methods</h4>\n");
            for m in &st.methods {
                html.push_str(&render_code_block(&m.signature));
                if !m.doc.is_empty() {
                    html.push_str("  <div class=\"doc\">\n");
                    html.push_str(&render_markdown_html(&m.doc));
                    html.push_str("  </div>\n");
                }
            }
        }
        html.push_str("</section>\n");
    }
}

fn render_html_enums(html: &mut String, module: &DocModule) {
    if module.enums.is_empty() {
        return;
    }
    html.push_str("<h2 class=\"sect\">Enums</h2>\n");
    for e in &module.enums {
        html.push_str(&format!(
            "<section class=\"fn doc-item\" id=\"en-{}\">\n",
            escape_html(&e.name)
        ));
        html.push_str(&format!(
            "  <h2><a href=\"#en-{}\"><code>{}</code></a>{} <span class=\"badge badge-outline\">enum</span></h2>\n",
            escape_html(&e.name),
            escape_html(&e.name),
            if e.is_pub {
                " <span class=\"badge badge-pub\">pub</span>"
            } else {
                ""
            }
        ));
        if !e.doc.is_empty() {
            html.push_str("  <div class=\"doc\">\n");
            html.push_str(&render_markdown_html(&e.doc));
            html.push_str("  </div>\n");
        }
        if !e.variants.is_empty() {
            html.push_str("  <h4>Variants</h4>\n");
            html.push_str("  <div class=\"tblwrap\"><table class=\"tbl\">\n");
            html.push_str("    <tr><th>Variant</th><th>Value</th></tr>\n");
            for v in &e.variants {
                let val = v.value.as_deref().unwrap_or("-");
                html.push_str(&format!(
                    "    <tr><td><code class=\"inline\">{}</code></td><td>{}</td></tr>\n",
                    escape_html(&v.name),
                    escape_html(val)
                ));
            }
            html.push_str("  </table></div>\n");
        }
        html.push_str("</section>\n");
    }
}

fn render_html_functions(html: &mut String, module: &DocModule) {
    if module.functions.is_empty() {
        return;
    }
    html.push_str("<h2 class=\"sect\">Functions</h2>\n");
    for f in &module.functions {
        html.push_str(&format!(
            "<section class=\"fn doc-item\" id=\"fn-{}\">\n",
            escape_html(&f.name)
        ));
        html.push_str(&format!(
            "  <h2><a href=\"#fn-{}\"><code>{}</code></a>{}</h2>\n",
            escape_html(&f.name),
            escape_html(&f.name),
            if f.is_pub {
                " <span class=\"badge badge-pub\">pub</span>"
            } else {
                ""
            }
        ));
        html.push_str(&render_code_block(&f.signature));
        if !f.doc.is_empty() {
            html.push_str("  <div class=\"doc\">\n");
            html.push_str(&render_markdown_html(&f.doc));
            html.push_str("  </div>\n");
        }
        if !f.params.is_empty() {
            html.push_str("  <h4>Parameters</h4>\n");
            html.push_str("  <div class=\"tblwrap\"><table class=\"tbl\">\n");
            html.push_str("    <tr><th>Parameter</th><th>Type</th><th>Default</th></tr>\n");
            for p in &f.params {
                let ty = p.type_ann.as_deref().unwrap_or("auto");
                let def = p.default_val.as_deref().unwrap_or("-");
                html.push_str(&format!(
                    "    <tr><td><code class=\"inline\">{}</code></td><td><code class=\"inline\">{}</code></td><td>{}</td></tr>\n",
                    escape_html(&p.name),
                    escape_html(ty),
                    escape_html(def)
                ));
            }
            html.push_str("  </table></div>\n");
        }
        if let Some(ref ret) = f.return_type {
            html.push_str(&format!(
                "  <div class=\"alert alert-ok\"><span class=\"ico\">\u{21A9}</span><div><b>Returns</b><p><code class=\"inline\">{}</code></p></div></div>\n",
                escape_html(ret)
            ));
        }
        html.push_str("</section>\n");
    }
}

/**
 * Generate a modern, categorized documentation hub / portal (index.html).
 */
pub fn generate_index_html(modules: &[DocModule], pkg_name: Option<&str>) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"light\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    if let Some(pkg) = pkg_name {
        html.push_str(&format!(
            "<title>{} - API Documentation</title>\n",
            escape_html(pkg)
        ));
    } else {
        html.push_str("<title>Alya Standard Library - Documentation</title>\n");
    }
    html.push_str("<script>try{if(localStorage.getItem('alya-docs-theme')==='dark')document.documentElement.dataset.theme='dark';}catch(e){}</script>\n");
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str("</style>\n</head>\n<body>\n");

    // Topbar shared with module pages.
    html.push_str("<header class=\"topbar\">\n  <div class=\"topbar-in\">\n");
    html.push_str(&format!(
        "    <button class=\"iconbtn hamb\" id=\"navToggle\" title=\"Modules\" aria-label=\"Toggle module navigation\" onclick=\"toggleDrawer('nav')\">{}</button>\n",
        MENU_SVG
    ));
    html.push_str("    <a class=\"brand\" href=\"index.html\">\n");
    html.push_str(ALYA_LOGO_SVG);
    html.push_str("      <b>Alya</b><span>Docs</span>\n    </a>\n");
    html.push_str(&format!(
        "    <span class=\"badge badge-outline\">v{}</span>\n",
        env!("CARGO_PKG_VERSION")
    ));
    html.push_str("    <label class=\"cmdk\">\u{2315} <input id=\"q\" type=\"search\" placeholder=\"Filter modules\u{2026}\" autocomplete=\"off\" oninput=\"filterModules()\"><kbd>\u{2318}K</kbd></label>\n");
    html.push_str("    <div class=\"topbar-right\">\n");
    html.push_str("    <button class=\"iconbtn\" id=\"themeBtn\" title=\"Toggle theme\" aria-label=\"Toggle theme\">\n");
    html.push_str(MOON_SVG);
    html.push_str(SUN_SVG);
    html.push_str("    </button>\n");
    html.push_str(&format!(
        "    <a class=\"iconbtn\" href=\"https://github.com/alya-lang/alya\" target=\"_blank\" rel=\"noopener noreferrer\" title=\"GitHub\" aria-label=\"GitHub repository\">{}</a>\n",
        GITHUB_LOGO_SVG
    ));
    html.push_str(&format!(
        "    <button class=\"iconbtn hamb\" id=\"tocToggle\" title=\"On this page\" aria-label=\"Toggle page contents\" onclick=\"toggleDrawer('toc')\">{}</button>\n",
        LIST_SVG
    ));
    html.push_str("    </div>\n");
    html.push_str("  </div>\n</header>\n");
    html.push_str("<div class=\"scrim\" id=\"scrim\" onclick=\"closeDrawers()\"></div>\n\n");

    // Group modules by category for sidebar, hero sections, and TOC.
    let mut groups: std::collections::BTreeMap<&str, Vec<&DocModule>> =
        std::collections::BTreeMap::new();
    for m in modules {
        let stem = m.name.rsplit('/').next().unwrap_or(&m.name);
        let (cat, _, _) = categorize_module(stem);
        groups.entry(cat).or_default().push(m);
    }

    html.push_str("<div class=\"shell\">\n\n");

    // Sidebar: category groups with per-module links.
    html.push_str("<aside class=\"sidenav\">\n");
    for (cat, members) in &groups {
        html.push_str("  <details class=\"side-group\" open>\n");
        html.push_str(&format!(
            "    <summary>{} <span class=\"n\">{}</span></summary>\n",
            escape_html(cat),
            members.len()
        ));
        for m in members {
            let target = format!("{}.html", m.name.replace('/', "_"));
            html.push_str(&format!(
                "    <a href=\"{}\"><code>{}</code></a>\n",
                target,
                escape_html(&m.name)
            ));
        }
        html.push_str("  </details>\n");
    }
    html.push_str("</aside>\n\n");

    // Main: hero plus one grouped card grid per category.
    html.push_str("<main class=\"content\">\n");
    html.push_str("  <div class=\"hero\">\n");
    if let Some(pkg) = pkg_name {
        html.push_str(&format!(
            "    <h1>{} <span class=\"grad\">Reference</span></h1>\n",
            escape_html(pkg)
        ));
        html.push_str(&format!(
            "    <p>{} modules. <code class=\"inline\">alya doc</code> generated API reference.</p>\n",
            modules.len()
        ));
    } else {
        html.push_str("    <h1>Standard Library <span class=\"grad\">Reference</span></h1>\n");
        html.push_str(&format!(
            "    <p>{} embedded modules, zero dependencies. <code class=\"inline\">import \"std/name\"</code> and go.</p>\n",
            modules.len()
        ));
    }
    html.push_str("  </div>\n");
    for (cat, members) in &groups {
        let anchor = cat.to_lowercase().replace(' ', "-").replace('&', "and");
        html.push_str(&format!(
            "  <section class=\"modgroup\" data-group id=\"g-{}\">\n    <h2>{}</h2>\n    <div class=\"modgrid\">\n",
            escape_html(&anchor),
            escape_html(cat)
        ));
        for m in members {
            let target = format!("{}.html", m.name.replace('/', "_"));
            let desc = if !m.description.is_empty() {
                m.description
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                    .unwrap_or(&m.description)
                    .to_string()
            } else {
                get_module_fallback_desc(&m.name)
            };
            let total_symbols = m.functions.len()
                + m.structs.len()
                + m.enums.len()
                + m.interfaces.len()
                + m.constants.len();
            html.push_str(&format!(
                "      <a class=\"modcard\" href=\"{}\" data-name=\"{}\"><code>{}</code><p>{}</p><span class=\"badge badge-secondary\">{} symbols</span></a>\n",
                target,
                escape_html(&m.name.to_lowercase()),
                escape_html(&m.name),
                escape_html(&desc),
                total_symbols
            ));
        }
        html.push_str("    </div>\n  </section>\n");
    }
    html.push_str("</main>\n\n");

    // Right rail: group anchors.
    html.push_str("<aside class=\"toc\" id=\"toc\">\n  <h5>On This Page</h5>\n");
    for cat in groups.keys() {
        let anchor = cat.to_lowercase().replace(' ', "-").replace('&', "and");
        html.push_str(&format!(
            "  <a href=\"#g-{}\">{}</a>\n",
            escape_html(&anchor),
            escape_html(cat)
        ));
    }
    html.push_str("</aside>\n\n");

    html.push_str("</div>\n\n");

    html.push_str("<footer class=\"footer\">\n");
    if let Some(pkg) = pkg_name {
        html.push_str(&format!(
            "  <span>Generated with <code class=\"inline\">alya doc</code> &middot; {} API reference</span>\n",
            escape_html(pkg)
        ));
    } else {
        html.push_str(&format!(
            "  <span>Generated with <code class=\"inline\">alya doc</code> &middot; Alya Standard Library v{}</span>\n",
            env!("CARGO_PKG_VERSION")
        ));
    }
    html.push_str("</footer>\n\n");

    // Client-side scripts: theme, ⌘K focus, live card filter, scroll-spy.
    html.push_str(
        r##"<script>
try{document.documentElement.dataset.theme=localStorage.getItem('alya-docs-theme')||'light';}catch(e){}
document.getElementById('themeBtn').addEventListener('click', () => {
  const el = document.documentElement;
  el.dataset.theme = el.dataset.theme === 'dark' ? 'light' : 'dark';
  try{localStorage.setItem('alya-docs-theme', el.dataset.theme);}catch(e){}
});
document.addEventListener('keydown', (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    const q = document.getElementById('q');
    if (q) q.focus();
  }
});
function filterModules() {
  const input = document.getElementById('q');
  const q = input ? input.value.toLowerCase().trim() : '';
  document.querySelectorAll('.modcard').forEach((card) => {
    const text = (card.dataset.name + ' ' + card.textContent).toLowerCase();
    card.style.display = !q || text.includes(q) ? '' : 'none';
  });
  document.querySelectorAll('[data-group]').forEach((g) => {
    const any = [...g.querySelectorAll('.modcard')].some((c) => c.style.display !== 'none');
    g.style.display = any ? '' : 'none';
  });
}
const tocLinks = [...document.querySelectorAll('#toc a')];
const spy = new IntersectionObserver((entries) => {
  entries.forEach((en) => {
    if (en.isIntersecting) {
      tocLinks.forEach((a) => a.classList.toggle('on', a.getAttribute('href') === '#' + en.target.id));
    }
  });
}, { rootMargin: '-20% 0px -70% 0px' });
tocLinks.forEach((a) => {
  const s = document.querySelector(a.getAttribute('href'));
  if (s) spy.observe(s);
});
function toggleDrawer(which) {
  const left = document.querySelector('.sidenav');
  const right = document.querySelector('.toc');
  const scrim = document.getElementById('scrim');
  const wantLeft = which === 'nav' ? !left.classList.contains('open') : false;
  const wantRight = which === 'toc' && right ? !right.classList.contains('open') : false;
  left.classList.toggle('open', wantLeft);
  if (right) right.classList.toggle('open', wantRight);
  if (scrim) scrim.classList.toggle('open', wantLeft || wantRight);
}
function closeDrawers() {
  document.querySelectorAll('.sidenav.open, .toc.open').forEach((el) => el.classList.remove('open'));
  const scrim = document.getElementById('scrim');
  if (scrim) scrim.classList.remove('open');
}
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeDrawers();
});
document.querySelectorAll('.sidenav a').forEach((a) => {
  a.addEventListener('click', () => closeDrawers());
});
</script>
</body>
</html>
"##,
    );

    html
}

fn categorize_module(name: &str) -> (&'static str, &'static str, &'static str) {
    match name {
        "math" | "str" | "mem" | "time" | "rand" => ("Core & Math", "badge-func", "Core"),
        "collections" | "json" | "hash" => ("Collections & Data", "badge-struct", "Data"),
        "fs" | "path" | "io" | "glob" => ("I/O & File System", "badge-interface", "IO"),
        "os" | "process" | "cli" | "console" | "color" => ("System & OS", "badge-amber", "System"),
        "sync" | "thread" | "net" => ("Concurrency & Net", "badge-pub", "Concurrency"),
        "test" | "bench" | "log" => ("Testing & Tooling", "badge-const", "Tooling"),
        _ => ("Module", "badge-kind", "all"),
    }
}

fn get_module_fallback_desc(name: &str) -> String {
    match name {
        "math" => "Mathematical constants, trigonometry, power, root, and statistical operations.",
        "str" => "String manipulation, UTF-8 unicode processing, slicing, and pattern utilities.",
        "mem" => "Low-level pointer operations, heap allocation statistics, and memory models.",
        "time" => "High-resolution monotonic timestamps, durations, sleep, and clock operations.",
        "rand" => "Cryptographically secure and pseudorandom number generators and distributions.",
        "collections" => "Advanced data structures: HashMap, HashSet, Vector, RingBuffer, and Deque.",
        "json" => "Fast recursive-descent JSON serializer and parser for structured Alya types.",
        "hash" => "Cryptographic and non-cryptographic hashing: SHA-256, MD5, FNV-1a, and SipHash.",
        "fs" => "File system operations: atomic reads, buffered writes, permissions, and directory walks.",
        "path" => "Cross-platform filesystem path manipulation, normalization, and extension helpers.",
        "io" => "Abstract stream primitives: Readers, Writers, Buffered IO, and pipes.",
        "glob" => "Pattern matching engine for filesystem path searching and wildcard resolution.",
        "os" => "Operating system abstraction: environment variables, hostname, and CPU cores.",
        "process" => "Subprocess spawning, IPC pipes, exit codes, and signal management.",
        "cli" => "Command-line argument parser, flag definitions, and automatic help generation.",
        "console" => "Interactive terminal input/output, prompts, and formatted text.",
        "color" => "TrueColor ANSI terminal text styler, 24-bit RGB palettes, and styles.",
        "sync" => "Colorless concurrency synchronization: Mutex, Channel, Condvar, and Atomic.",
        "thread" => "Native OS thread spawning, fiber dispatch, and worker threadpools.",
        "net" => "TCP/UDP socket client and server primitives with non-blocking polling.",
        "test" => "Native test runner, assertions, and test suite execution framework.",
        "bench" => "High-precision micro-benchmarking harness with latency measurements.",
        "log" => "Structured leveled logging (trace, debug, info, warn, error) with timestamps.",
        _ => "Standard library module providing foundational Alya types and utilities.",
    }.to_string()
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/**
 * Renders a signature block with syntax highlighting and a floating copy button.
 * Copying is handled by the delegated `.copy` click listener in page scripts.
 */
fn render_code_block(sig: &str) -> String {
    let highlighted = highlight_signature(sig);
    format!(
        "<div class=\"codeblock\">{}\n<button class=\"copy\">Copy</button></div>\n",
        highlighted
    )
}

/**
 * Lightweight syntax highlighter for Alya signatures.
 */
fn highlight_signature(sig: &str) -> String {
    let escaped = escape_html(sig);

    // Keywords
    let keywords = [
        "pub ",
        "function ",
        "fn ",
        "struct ",
        "interface ",
        "enum ",
        "const ",
        "let ",
        "return ",
        "end",
        "when ",
        "if ",
        "else ",
        "while ",
        "for ",
        "in ",
        "spawn ",
        "defer ",
    ];

    let mut res = escaped;

    for kw in &keywords {
        let replacement = format!("<span class=\"kw\">{}</span>", kw.trim_end());
        if kw.ends_with(' ') {
            res = res.replace(kw, &format!("{} ", replacement));
        } else {
            res = res.replace(kw, &replacement);
        }
    }

    // Return arrow
    res = res.replace("-&gt;", "<span class=\"arr\">-&gt;</span>");

    // Primitive types
    let types = [
        "float", "int", "string", "bool", "void", "char", "byte", "any", "Point", "Vector2D",
    ];
    for ty in &types {
        let pattern = format!(": {}", ty);
        let replacement = format!(": <span class=\"ty\">{}</span>", ty);
        res = res.replace(&pattern, &replacement);

        let ret_pattern = format!("-&gt;</span> {}", ty);
        let ret_replacement = format!("-&gt;</span> <span class=\"ty\">{}</span>", ty);
        res = res.replace(&ret_pattern, &ret_replacement);
    }

    res
}

/**
 * Rich Markdown renderer with inline code, bold, lists, and section recognition.
 */
fn render_markdown_html(md: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;
    let mut in_list = false;

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                out.push_str("</code></pre></div>\n");
                in_code_block = false;
            } else {
                if in_list {
                    out.push_str("</ul>\n");
                    in_list = false;
                }
                out.push_str(
                    "<div class=\"codeblock\"><button class=\"copy\">Copy</button><pre><code>",
                );
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            out.push_str(&escape_html(line));
            out.push('\n');
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("### ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            if title.eq_ignore_ascii_case("parameters") {
                out.push_str("<div class=\"section-label\">Parameters</div>\n");
            } else if title.eq_ignore_ascii_case("returns") {
                out.push_str("<div class=\"section-label\">Returns</div>\n");
            } else if title.eq_ignore_ascii_case("throws") {
                out.push_str("<div class=\"section-label\" style=\"color: var(--accent-rose);\">⚠️ Throws</div>\n");
            } else {
                out.push_str(&format!("<h4 style=\"font-size: 1.1rem; color: var(--accent); margin: 16px 0 8px;\">{}</h4>\n", escape_html(title)));
            }
            continue;
        }

        if let Some(content) = trimmed.strip_prefix("## ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!(
                "<h3 style=\"font-size: 1.25rem; color: var(--fg); margin: 20px 0 10px;\">{}</h3>\n",
                escape_html(content)
            ));
            continue;
        }

        if let Some(content) = trimmed.strip_prefix("# ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!(
                "<h2 style=\"font-size: 1.5rem; color: var(--fg); margin: 24px 0 12px;\">{}</h2>\n",
                escape_html(content)
            ));
            continue;
        }

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if !in_list {
                out.push_str("<ul style=\"margin: 8px 0 14px 20px; color: var(--fg);\">\n");
                in_list = true;
            }
            let item_text = &trimmed[2..];
            out.push_str(&format!(
                "  <li style=\"margin: 4px 0;\">{}</li>\n",
                format_inline_markdown(item_text)
            ));
            continue;
        }

        if in_list {
            out.push_str("</ul>\n");
            in_list = false;
        }

        if trimmed.is_empty() {
            continue;
        }

        out.push_str(&format!(
            "<p style=\"margin-bottom: 12px; color: var(--fg); font-size: 0.95rem;\">{}</p>\n",
            format_inline_markdown(trimmed)
        ));
    }

    if in_list {
        out.push_str("</ul>\n");
    }
    if in_code_block {
        out.push_str("</code></pre></div>\n");
    }

    out
}

/**
 * Format inline markdown: `code`, **bold**, *italic*.
 */
fn format_inline_markdown(text: &str) -> String {
    let mut res = escape_html(text);

    // Inline code `...`
    while let Some(start) = res.find('`') {
        if let Some(end) = res[start + 1..].find('`') {
            let actual_end = start + 1 + end;
            let inside = &res[start + 1..actual_end];
            let pill = format!("<code class=\"inline-code\">{}</code>", inside);
            res = format!("{}{}{}", &res[..start], pill, &res[actual_end + 1..]);
        } else {
            break;
        }
    }

    // Bold **...**
    while let Some(start) = res.find("**") {
        if let Some(end) = res[start + 2..].find("**") {
            let actual_end = start + 2 + end;
            let inside = &res[start + 2..actual_end];
            let bold = format!("<strong>{}</strong>", inside);
            res = format!("{}{}{}", &res[..start], bold, &res[actual_end + 2..]);
        } else {
            break;
        }
    }

    res
}
