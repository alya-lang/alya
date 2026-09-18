use super::extractor::DocModule;

// Embedded official Alya 3D Delta Prism vector logo
const ALYA_LOGO_SVG: &str = r##"<svg class="alya-logo" width="28" height="28" viewBox="0 0 485 512" fill="none" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="logo-left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#c084fc"/>
      <stop offset="100%" stop-color="#7e22ce"/>
    </linearGradient>
    <linearGradient id="logo-right" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#38bdf8"/>
      <stop offset="100%" stop-color="#0284c7"/>
    </linearGradient>
    <linearGradient id="logo-glow" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#38bdf8"/>
    </linearGradient>
  </defs>
  <path d="M242.5 32 L40 440 L242.5 360 Z" fill="url(#logo-left)" opacity="0.95"/>
  <path d="M242.5 32 L445 440 L242.5 360 Z" fill="url(#logo-right)" opacity="0.95"/>
  <path d="M242.5 32 L180 375 L242.5 360 Z" fill="url(#logo-glow)" opacity="0.4"/>
  <path d="M242.5 32 L305 375 L242.5 360 Z" fill="#ffffff" opacity="0.6"/>
  <polygon points="242.5,30 252,50 242.5,45 233,50" fill="#ffffff"/>
</svg>"##;

// Official GitHub vector icon from svgl.app
const GITHUB_LOGO_SVG: &str = r##"<svg class="github-icon" width="20" height="20" viewBox="0 0 1024 1024" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path fill-rule="evenodd" clip-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8C0 11.54 2.29 14.53 5.47 15.59C5.87 15.66 6.02 15.42 6.02 15.21C6.02 15.02 6.01 14.39 6.01 13.72C4 14.09 3.48 13.23 3.32 12.78C3.23 12.55 2.84 11.84 2.5 11.65C2.22 11.5 1.82 11.13 2.49 11.12C3.12 11.11 3.57 11.7 3.72 11.94C4.44 13.15 5.59 12.81 6.05 12.6C6.12 12.08 6.33 11.73 6.56 11.53C4.78 11.33 2.92 10.64 2.92 7.58C2.92 6.71 3.23 5.99 3.74 5.43C3.66 5.23 3.38 4.41 3.82 3.31C3.82 3.31 4.49 3.1 6.02 4.13C6.66 3.95 7.34 3.86 8.02 3.86C8.7 3.86 9.38 3.95 10.02 4.13C11.55 3.09 12.22 3.31 12.22 3.31C12.66 4.41 12.38 5.23 12.3 5.43C12.81 5.99 13.12 6.7 13.12 7.58C13.12 10.65 11.25 11.33 9.47 11.53C9.76 11.78 10.01 12.26 10.01 13.01C10.01 14.08 10 14.94 10 15.21C10 15.42 10.15 15.67 10.55 15.59C13.71 14.53 16 11.53 16 8C16 3.58 12.42 0 8 0Z" transform="scale(64)" fill="currentColor"/>
</svg>"##;

const COMMON_CSS: &str = r##"
:root {
  --bg: #080c16;
  --bg-surface: rgba(13, 19, 35, 0.75);
  --bg-card: rgba(18, 26, 46, 0.65);
  --bg-card-hover: rgba(22, 33, 58, 0.85);
  --bg-code: #050811;
  --border: rgba(255, 255, 255, 0.08);
  --border-hover: rgba(56, 189, 248, 0.4);
  --border-glow: rgba(192, 132, 252, 0.3);
  --text: #f8fafc;
  --text-muted: #94a3b8;
  --text-dim: #64748b;
  --accent: #38bdf8;
  --accent-purple: #c084fc;
  --accent-emerald: #10b981;
  --accent-amber: #f59e0b;
  --accent-rose: #f43f5e;
  --grad-brand: linear-gradient(135deg, #c084fc 0%, #38bdf8 100%);
  --grad-header: linear-gradient(135deg, #ffffff 20%, #cbd5e1 100%);
  --shadow-card: 0 4px 20px -2px rgba(0, 0, 0, 0.4);
  --shadow-glow: 0 0 25px -5px rgba(56, 189, 248, 0.15);
}

* { box-sizing: border-box; margin: 0; padding: 0; }

/* Custom Dark Scrollbars */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: rgba(8, 12, 22, 0.6);
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.14);
  border-radius: 9999px;
  border: 2px solid rgba(8, 12, 22, 0.6);
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(56, 189, 248, 0.45);
}

* {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.16) rgba(8, 12, 22, 0.6);
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background-color: var(--bg);
  background-image: 
    radial-gradient(circle at 12% 15%, rgba(192, 132, 252, 0.08) 0%, transparent 45%),
    radial-gradient(circle at 88% 25%, rgba(56, 189, 248, 0.07) 0%, transparent 45%),
    radial-gradient(circle at 50% 85%, rgba(16, 185, 129, 0.04) 0%, transparent 50%);
  background-attachment: fixed;
  color: var(--text);
  line-height: 1.6;
  min-height: 100vh;
}

/* Header */
.top-nav {
  position: sticky;
  top: 0;
  z-index: 100;
  height: 64px;
  background: rgba(8, 12, 22, 0.85);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 32px;
}

.brand-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
  text-decoration: none;
  color: inherit;
}

.brand-title {
  font-size: 1.25rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  background: var(--grad-brand);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.brand-sub {
  font-size: 0.85rem;
  color: var(--text-muted);
  font-weight: 500;
  margin-left: 2px;
}

.version-badge, .version-pill {
  display: inline-flex;
  align-items: center;
  padding: 3px 9px;
  background: rgba(56, 189, 248, 0.12);
  border: 1px solid rgba(56, 189, 248, 0.35);
  color: #38bdf8 !important;
  border-radius: 9999px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.03em;
  line-height: 1;
  text-decoration: none !important;
}

.nav-links {
  display: flex;
  align-items: center;
  gap: 16px;
}

.nav-link {
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.9rem;
  font-weight: 500;
  transition: all 0.2s ease;
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.nav-link:hover {
  color: #fff;
}

.github-link {
  padding: 6px 12px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border);
}

.github-link:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.2);
  color: #fff;
}

.github-icon {
  display: inline-block;
  vertical-align: middle;
}

/* Search Box */
.search-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.search-input {
  width: 100%;
  padding: 10px 14px 10px 36px;
  background: var(--bg-code);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  font-size: 0.9rem;
  transition: all 0.2s ease;
}

.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(56, 189, 248, 0.15);
}

.search-icon {
  position: absolute;
  left: 12px;
  color: var(--text-dim);
  font-size: 0.9rem;
  pointer-events: none;
}

/* Badges */
.badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.badge-pub {
  background: rgba(16, 185, 129, 0.12);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.badge-kind {
  background: rgba(255, 255, 255, 0.06);
  color: #cbd5e1;
  border: 1px solid var(--border);
}

.badge-func {
  background: rgba(56, 189, 248, 0.12);
  color: #38bdf8;
  border: 1px solid rgba(56, 189, 248, 0.3);
}

.badge-struct {
  background: rgba(192, 132, 252, 0.12);
  color: #c084fc;
  border: 1px solid rgba(192, 132, 252, 0.3);
}

.badge-enum {
  background: rgba(245, 158, 11, 0.12);
  color: #fbbf24;
  border: 1px solid rgba(245, 158, 11, 0.3);
}

.badge-interface {
  background: rgba(99, 102, 241, 0.12);
  color: #a5b4fc;
  border: 1px solid rgba(99, 102, 241, 0.3);
}

.badge-const {
  background: rgba(244, 63, 94, 0.12);
  color: #fb7185;
  border: 1px solid rgba(244, 63, 94, 0.3);
}

/* Code Blocks with Copy Button */
.code-container {
  position: relative;
  margin: 14px 0 18px;
}

pre {
  background: var(--bg-code);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 8px;
  padding: 16px 20px;
  overflow-x: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.92rem;
  line-height: 1.5;
  color: #e2e8f0;
}

.copy-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  padding: 4px 10px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-muted);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  opacity: 0;
  transition: all 0.15s ease;
}

.code-container:hover .copy-btn {
  opacity: 1;
}

.copy-btn:hover {
  background: rgba(56, 189, 248, 0.15);
  color: var(--accent);
  border-color: rgba(56, 189, 248, 0.4);
}

/* Syntax Highlighting */
.kw { color: #c084fc; font-weight: 600; }
.ident { color: #38bdf8; }
.ty { color: #fbbf24; }
.arr { color: #94a3b8; }
.str { color: #34d399; }
.num { color: #f43f5e; }
.cm { color: #64748b; font-style: italic; }

code.inline-code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: rgba(56, 189, 248, 0.08);
  border: 1px solid rgba(56, 189, 248, 0.18);
  color: #7dd3fc;
  padding: 2px 7px;
  border-radius: 4px;
  font-size: 0.88em;
}

/* Doc Cards */
.card {
  background: var(--bg-card);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  margin-bottom: 24px;
  box-shadow: var(--shadow-card);
  transition: all 0.2s ease;
}

.card:hover {
  border-color: var(--border-hover);
  box-shadow: var(--shadow-glow);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 8px;
}

.card-title {
  font-size: 1.35rem;
  color: #fff;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
}

.card-title a.anchor {
  color: var(--text-dim);
  text-decoration: none;
  opacity: 0;
  font-size: 1rem;
  transition: opacity 0.15s ease;
}

.card:hover .card-title a.anchor {
  opacity: 1;
}

.card-title a.anchor:hover {
  color: var(--accent);
}

.section-label {
  font-size: 0.85rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin: 18px 0 8px;
}

/* Modern Parameter & Property Table */
.param-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  margin: 10px 0 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}

.param-table th {
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-muted);
  font-weight: 600;
  font-size: 0.82rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.param-table td {
  padding: 10px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  font-size: 0.9rem;
}

.param-table tr:last-child td {
  border-bottom: none;
}

/* Alerts / Callouts */
.alert {
  padding: 14px 18px;
  border-radius: 8px;
  margin: 14px 0;
  font-size: 0.92rem;
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.alert-throws {
  background: rgba(244, 63, 94, 0.08);
  border: 1px solid rgba(244, 63, 94, 0.25);
  color: #fecdd3;
}

.alert-throws strong {
  color: #fda4af;
}

.return-block {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  padding: 6px 14px;
  background: rgba(56, 189, 248, 0.06);
  border: 1px solid rgba(56, 189, 248, 0.2);
  border-radius: 6px;
  font-size: 0.92rem;
}

.return-block .arrow {
  color: var(--accent);
  font-weight: bold;
}
"##;

pub fn generate_html(module: &DocModule) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!(
        "<title>Module {} - Alya Docs</title>\n",
        escape_html(&module.name)
    ));
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str(
        r##"
.module-layout {
  display: flex;
  min-height: calc(100vh - 64px);
}

aside {
  width: 290px;
  background: rgba(10, 15, 28, 0.85);
  backdrop-filter: blur(12px);
  border-right: 1px solid var(--border);
  position: sticky;
  top: 64px;
  height: calc(100vh - 64px);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

.aside-header {
  padding: 20px 18px 14px;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.back-link {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  gap: 8px;
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.85rem;
  font-weight: 600;
  margin-bottom: 14px;
  padding: 8px 12px;
  border-radius: 8px;
  transition: all 0.15s ease;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border);
  box-sizing: border-box;
}

.back-link:hover {
  color: var(--accent);
  border-color: rgba(56, 189, 248, 0.35);
  background: rgba(56, 189, 248, 0.08);
}

.aside-header h2 {
  font-size: 1.15rem;
  color: #fff;
  margin-bottom: 12px;
  font-weight: 700;
}

.aside-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 18px 24px;
  scrollbar-width: thin;
  scrollbar-color: rgba(56, 189, 248, 0.25) transparent;
}

.aside-content::-webkit-scrollbar {
  width: 6px;
}

.aside-content::-webkit-scrollbar-track {
  background: transparent;
}

.aside-content::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 9999px;
}

.aside-content::-webkit-scrollbar-thumb:hover {
  background: rgba(56, 189, 248, 0.4);
}

.aside-content ul { list-style: none; }
.aside-content li { margin: 4px 0; }

.aside-content a {
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-radius: 6px;
  transition: all 0.15s ease;
}

.aside-content a:hover {
  background: rgba(56, 189, 248, 0.1);
  color: var(--accent);
}

.nav-group {
  margin-bottom: 22px;
}

.nav-group-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-dim);
  margin-bottom: 8px;
  padding-left: 6px;
}

.main-wrapper {
  flex: 1;
  min-width: 0;
  display: flex;
  justify-content: center;
  padding: 0 48px;
}

main {
  width: 100%;
  max-width: 1120px;
  padding: 40px 0 80px;
}

.module-header {
  margin-bottom: 36px;
  border-bottom: 1px solid var(--border);
  padding-bottom: 24px;
}

.module-title {
  font-size: 2.4rem;
  font-weight: 800;
  letter-spacing: -0.03em;
  background: var(--grad-header);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin-bottom: 10px;
}

.module-meta {
  display: flex;
  align-items: center;
  gap: 16px;
  color: var(--text-muted);
  font-size: 0.9rem;
  margin-bottom: 16px;
}

.module-desc {
  color: #cbd5e1;
  font-size: 1.05rem;
  line-height: 1.7;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 12px;
  margin-top: 20px;
}

.summary-pill {
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border);
  border-radius: 8px;
  text-align: center;
}

.summary-pill .num {
  font-size: 1.3rem;
  font-weight: 700;
  color: #fff;
}

.summary-pill .label {
  font-size: 0.75rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

@media (max-width: 860px) {
  .module-layout { flex-direction: column; }
  aside { width: 100%; height: auto; position: static; }
  .aside-content { overflow-y: visible; height: auto; }
  .main-wrapper { padding: 0 20px; }
  main { padding: 24px 0 60px; }
}
</style>
</head>
<body>
"##,
    );

    // Top navigation bar
    html.push_str("<header class=\"top-nav\">\n");
    html.push_str("  <a href=\"index.html\" class=\"brand-wrapper\">\n");
    html.push_str(ALYA_LOGO_SVG);
    html.push_str("    <span class=\"brand-title\">Alya</span>\n");
    html.push_str("    <span class=\"brand-sub\">Docs</span>\n");
    html.push_str(&format!(
        "    <span class=\"version-badge\">v{}</span>\n",
        env!("CARGO_PKG_VERSION")
    ));
    html.push_str("  </a>\n");
    html.push_str("  <nav class=\"nav-links\">\n");
    html.push_str(&format!(
        "    <a href=\"https://github.com/alya-lang/alya\" target=\"_blank\" rel=\"noopener noreferrer\" class=\"nav-link github-link\" aria-label=\"GitHub\">\n      {}\n      <span>GitHub</span>\n    </a>\n",
        GITHUB_LOGO_SVG
    ));
    html.push_str("  </nav>\n");
    html.push_str("</header>\n\n");

    html.push_str("<div class=\"module-layout\">\n\n");

    // Sidebar
    html.push_str("<aside>\n");
    html.push_str("  <div class=\"aside-header\">\n");
    html.push_str("    <a href=\"index.html\" class=\"back-link\">← Back to All Modules</a>\n");
    html.push_str(&format!("    <h2>{}</h2>\n", escape_html(&module.name)));
    html.push_str("    <div class=\"search-wrapper\">\n");
    html.push_str("      <span class=\"search-icon\">🔍</span>\n");
    html.push_str("      <input type=\"text\" id=\"search\" class=\"search-input\" placeholder=\"Filter symbols...\" oninput=\"filterSymbols()\">\n");
    html.push_str("    </div>\n");
    html.push_str("  </div>\n");
    html.push_str("  <div class=\"aside-content\">\n");

    if !module.constants.is_empty() {
        html.push_str(
            "  <div class=\"nav-group\"><div class=\"nav-group-title\">Constants</div><ul>\n",
        );
        for c in &module.constants {
            html.push_str(&format!(
                "    <li class=\"nav-item\"><a href=\"#c-{}\"><span>{}</span><span class=\"badge badge-const\">c</span></a></li>\n",
                escape_html(&c.name),
                escape_html(&c.name)
            ));
        }
        html.push_str("  </ul></div>\n");
    }

    if !module.interfaces.is_empty() {
        html.push_str(
            "  <div class=\"nav-group\"><div class=\"nav-group-title\">Interfaces</div><ul>\n",
        );
        for iface in &module.interfaces {
            html.push_str(&format!(
                "    <li class=\"nav-item\"><a href=\"#if-{}\"><span>{}</span><span class=\"badge badge-interface\">if</span></a></li>\n",
                escape_html(&iface.name),
                escape_html(&iface.name)
            ));
        }
        html.push_str("  </ul></div>\n");
    }

    if !module.structs.is_empty() {
        html.push_str(
            "  <div class=\"nav-group\"><div class=\"nav-group-title\">Structs</div><ul>\n",
        );
        for st in &module.structs {
            html.push_str(&format!(
                "    <li class=\"nav-item\"><a href=\"#st-{}\"><span>{}</span><span class=\"badge badge-struct\">st</span></a></li>\n",
                escape_html(&st.name),
                escape_html(&st.name)
            ));
        }
        html.push_str("  </ul></div>\n");
    }

    if !module.enums.is_empty() {
        html.push_str(
            "  <div class=\"nav-group\"><div class=\"nav-group-title\">Enums</div><ul>\n",
        );
        for e in &module.enums {
            html.push_str(&format!(
                "    <li class=\"nav-item\"><a href=\"#en-{}\"><span>{}</span><span class=\"badge badge-enum\">en</span></a></li>\n",
                escape_html(&e.name),
                escape_html(&e.name)
            ));
        }
        html.push_str("  </ul></div>\n");
    }

    if !module.functions.is_empty() {
        html.push_str(
            "  <div class=\"nav-group\"><div class=\"nav-group-title\">Functions</div><ul>\n",
        );
        for f in &module.functions {
            html.push_str(&format!(
                "    <li class=\"nav-item\"><a href=\"#fn-{}\"><span>{}()</span><span class=\"badge badge-func\">fn</span></a></li>\n",
                escape_html(&f.name),
                escape_html(&f.name)
            ));
        }
        html.push_str("  </ul></div>\n");
    }
    html.push_str("  </div>\n"); // close aside-content
    html.push_str("</aside>\n\n");

    // Main content
    html.push_str("<div class=\"main-wrapper\">\n");
    html.push_str("<main>\n");

    // Module Header
    html.push_str("<div class=\"module-header\">\n");
    html.push_str(&format!(
        "  <h1 class=\"module-title\">std/{}</h1>\n",
        escape_html(&module.name)
    ));
    html.push_str("  <div class=\"module-meta\">\n");
    html.push_str(&format!(
        "    <span>Source: <code>{}</code></span>\n",
        escape_html(&module.file_path)
    ));
    html.push_str("  </div>\n");

    if !module.description.is_empty() {
        html.push_str("  <div class=\"module-desc\">\n");
        html.push_str(&render_markdown_html(&module.description));
        html.push_str("  </div>\n");
    }

    // Summary pills
    html.push_str("  <div class=\"summary-grid\">\n");
    if !module.functions.is_empty() {
        html.push_str(&format!(
            "    <div class=\"summary-pill\"><div class=\"num\">{}</div><div class=\"label\">Functions</div></div>\n",
            module.functions.len()
        ));
    }
    if !module.structs.is_empty() {
        html.push_str(&format!(
            "    <div class=\"summary-pill\"><div class=\"num\">{}</div><div class=\"label\">Structs</div></div>\n",
            module.structs.len()
        ));
    }
    if !module.interfaces.is_empty() {
        html.push_str(&format!(
            "    <div class=\"summary-pill\"><div class=\"num\">{}</div><div class=\"label\">Interfaces</div></div>\n",
            module.interfaces.len()
        ));
    }
    if !module.enums.is_empty() {
        html.push_str(&format!(
            "    <div class=\"summary-pill\"><div class=\"num\">{}</div><div class=\"label\">Enums</div></div>\n",
            module.enums.len()
        ));
    }
    if !module.constants.is_empty() {
        html.push_str(&format!(
            "    <div class=\"summary-pill\"><div class=\"num\">{}</div><div class=\"label\">Constants</div></div>\n",
            module.constants.len()
        ));
    }
    html.push_str("  </div>\n");
    html.push_str("</div>\n\n");

    // 1. Constants
    if !module.constants.is_empty() {
        html.push_str("<h2 style=\"font-size: 1.5rem; margin: 32px 0 16px; color: var(--accent-purple);\">Constants</h2>\n");
        for c in &module.constants {
            html.push_str(&format!(
                "<div class=\"card doc-item\" id=\"c-{}\">\n",
                escape_html(&c.name)
            ));
            html.push_str("  <div class=\"card-header\">\n");
            html.push_str("    <div class=\"card-title\">\n");
            html.push_str(&format!("      <span>{}</span>\n", escape_html(&c.name)));
            html.push_str(&format!(
                "      <a href=\"#c-{}\" class=\"anchor\">#</a>\n",
                escape_html(&c.name)
            ));
            html.push_str("    </div>\n");
            html.push_str("    <div>\n");
            if c.is_pub {
                html.push_str("      <span class=\"badge badge-pub\">pub</span>\n");
            }
            html.push_str("      <span class=\"badge badge-const\">const</span>\n");
            html.push_str("    </div>\n");
            html.push_str("  </div>\n");

            let sig = format!("pub const {} = {}", c.name, c.value);
            html.push_str(&render_code_block(&sig));

            if !c.doc.is_empty() {
                html.push_str(&render_markdown_html(&c.doc));
            }
            html.push_str("</div>\n");
        }
    }

    // 2. Interfaces
    if !module.interfaces.is_empty() {
        html.push_str("<h2 style=\"font-size: 1.5rem; margin: 32px 0 16px; color: var(--accent);\">Interfaces</h2>\n");
        for iface in &module.interfaces {
            html.push_str(&format!(
                "<div class=\"card doc-item\" id=\"if-{}\">\n",
                escape_html(&iface.name)
            ));
            html.push_str("  <div class=\"card-header\">\n");
            html.push_str("    <div class=\"card-title\">\n");
            html.push_str(&format!(
                "      <span>{}</span>\n",
                escape_html(&iface.name)
            ));
            html.push_str(&format!(
                "      <a href=\"#if-{}\" class=\"anchor\">#</a>\n",
                escape_html(&iface.name)
            ));
            html.push_str("    </div>\n");
            html.push_str("    <div>\n");
            if iface.is_pub {
                html.push_str("      <span class=\"badge badge-pub\">pub</span>\n");
            }
            html.push_str("      <span class=\"badge badge-interface\">interface</span>\n");
            html.push_str("    </div>\n");
            html.push_str("  </div>\n");

            if !iface.doc.is_empty() {
                html.push_str(&render_markdown_html(&iface.doc));
            }

            if !iface.methods.is_empty() {
                html.push_str("  <div class=\"section-label\">Interface Methods</div>\n");
                html.push_str("  <table class=\"param-table\">\n");
                html.push_str("    <thead><tr><th>Method</th><th>Parameters</th><th>Returns</th></tr></thead>\n    <tbody>\n");
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
                        "    <tr><td><code class=\"inline-code\">{}</code></td><td>{}</td><td><span class=\"badge badge-kind\">{}</span></td></tr>\n",
                        escape_html(&m.name),
                        escape_html(&param_str.join(", ")),
                        escape_html(ret)
                    ));
                }
                html.push_str("    </tbody>\n  </table>\n");
            }
            html.push_str("</div>\n");
        }
    }

    // 3. Structs
    if !module.structs.is_empty() {
        html.push_str("<h2 style=\"font-size: 1.5rem; margin: 32px 0 16px; color: var(--accent-purple);\">Structs</h2>\n");
        for st in &module.structs {
            html.push_str(&format!(
                "<div class=\"card doc-item\" id=\"st-{}\">\n",
                escape_html(&st.name)
            ));
            html.push_str("  <div class=\"card-header\">\n");
            html.push_str("    <div class=\"card-title\">\n");
            html.push_str(&format!("      <span>{}</span>\n", escape_html(&st.name)));
            html.push_str(&format!(
                "      <a href=\"#st-{}\" class=\"anchor\">#</a>\n",
                escape_html(&st.name)
            ));
            html.push_str("    </div>\n");
            html.push_str("    <div>\n");
            if st.is_pub {
                html.push_str("      <span class=\"badge badge-pub\">pub</span>\n");
            }
            html.push_str("      <span class=\"badge badge-struct\">struct</span>\n");
            html.push_str("    </div>\n");
            html.push_str("  </div>\n");

            if !st.doc.is_empty() {
                html.push_str(&render_markdown_html(&st.doc));
            }

            if !st.fields.is_empty() {
                html.push_str("  <div class=\"section-label\">Fields</div>\n");
                html.push_str("  <table class=\"param-table\">\n");
                html.push_str("    <thead><tr><th>Field</th><th>Type</th><th>Default</th></tr></thead>\n    <tbody>\n");
                for f in &st.fields {
                    let ty = f.type_ann.as_deref().unwrap_or("auto");
                    let def = f.default_val.as_deref().unwrap_or("-");
                    html.push_str(&format!(
                        "    <tr><td><code class=\"inline-code\">{}</code></td><td><code class=\"inline-code\">{}</code></td><td>{}</td></tr>\n",
                        escape_html(&f.name),
                        escape_html(ty),
                        escape_html(def)
                    ));
                }
                html.push_str("    </tbody>\n  </table>\n");
            }

            if !st.methods.is_empty() {
                html.push_str("  <div class=\"section-label\">Methods</div>\n");
                for m in &st.methods {
                    html.push_str(&render_code_block(&m.signature));
                    if !m.doc.is_empty() {
                        html.push_str(&render_markdown_html(&m.doc));
                    }
                }
            }
            html.push_str("</div>\n");
        }
    }

    // 4. Enums
    if !module.enums.is_empty() {
        html.push_str("<h2 style=\"font-size: 1.5rem; margin: 32px 0 16px; color: var(--accent-amber);\">Enums</h2>\n");
        for e in &module.enums {
            html.push_str(&format!(
                "<div class=\"card doc-item\" id=\"en-{}\">\n",
                escape_html(&e.name)
            ));
            html.push_str("  <div class=\"card-header\">\n");
            html.push_str("    <div class=\"card-title\">\n");
            html.push_str(&format!("      <span>{}</span>\n", escape_html(&e.name)));
            html.push_str(&format!(
                "      <a href=\"#en-{}\" class=\"anchor\">#</a>\n",
                escape_html(&e.name)
            ));
            html.push_str("    </div>\n");
            html.push_str("    <div>\n");
            if e.is_pub {
                html.push_str("      <span class=\"badge badge-pub\">pub</span>\n");
            }
            html.push_str("      <span class=\"badge badge-enum\">enum</span>\n");
            html.push_str("    </div>\n");
            html.push_str("  </div>\n");

            if !e.doc.is_empty() {
                html.push_str(&render_markdown_html(&e.doc));
            }

            if !e.variants.is_empty() {
                html.push_str("  <div class=\"section-label\">Variants</div>\n");
                html.push_str("  <table class=\"param-table\">\n");
                html.push_str("    <thead><tr><th>Variant</th><th>Value / Payload</th></tr></thead>\n    <tbody>\n");
                for v in &e.variants {
                    let val = v.value.as_deref().unwrap_or("-");
                    html.push_str(&format!(
                        "    <tr><td><code class=\"inline-code\">{}</code></td><td>{}</td></tr>\n",
                        escape_html(&v.name),
                        escape_html(val)
                    ));
                }
                html.push_str("    </tbody>\n  </table>\n");
            }
            html.push_str("</div>\n");
        }
    }

    // 5. Functions
    if !module.functions.is_empty() {
        html.push_str("<h2 style=\"font-size: 1.5rem; margin: 32px 0 16px; color: var(--accent);\">Functions</h2>\n");
        for f in &module.functions {
            html.push_str(&format!(
                "<div class=\"card doc-item\" id=\"fn-{}\">\n",
                escape_html(&f.name)
            ));
            html.push_str("  <div class=\"card-header\">\n");
            html.push_str("    <div class=\"card-title\">\n");
            html.push_str(&format!("      <span>{}</span>\n", escape_html(&f.name)));
            html.push_str(&format!(
                "      <a href=\"#fn-{}\" class=\"anchor\">#</a>\n",
                escape_html(&f.name)
            ));
            html.push_str("    </div>\n");
            html.push_str("    <div>\n");
            if f.is_pub {
                html.push_str("      <span class=\"badge badge-pub\">pub</span>\n");
            }
            html.push_str("      <span class=\"badge badge-func\">function</span>\n");
            html.push_str("    </div>\n");
            html.push_str("  </div>\n");

            html.push_str(&render_code_block(&f.signature));

            if !f.doc.is_empty() {
                html.push_str(&render_markdown_html(&f.doc));
            }

            if !f.params.is_empty() {
                html.push_str("  <div class=\"section-label\">Parameters</div>\n");
                html.push_str("  <table class=\"param-table\">\n");
                html.push_str("    <thead><tr><th>Parameter</th><th>Type</th><th>Default</th></tr></thead>\n    <tbody>\n");
                for p in &f.params {
                    let ty = p.type_ann.as_deref().unwrap_or("auto");
                    let def = p.default_val.as_deref().unwrap_or("-");
                    html.push_str(&format!(
                        "    <tr><td><code class=\"inline-code\">{}</code></td><td><code class=\"inline-code\">{}</code></td><td>{}</td></tr>\n",
                        escape_html(&p.name),
                        escape_html(ty),
                        escape_html(def)
                    ));
                }
                html.push_str("    </tbody>\n  </table>\n");
            }

            if let Some(ref ret) = f.return_type {
                html.push_str(&format!(
                    "  <div class=\"return-block\"><span class=\"arrow\">➔</span> <strong>Returns:</strong> <code class=\"inline-code\">{}</code></div>\n",
                    escape_html(ret)
                ));
            }

            html.push_str("</div>\n");
        }
    }

    html.push_str("</main>\n");
    html.push_str("</div>\n"); // close main-wrapper
    html.push_str("</div>\n"); // close module-layout

    // Client-side scripts (filter & copy)
    html.push_str(
        r##"<script>
function filterSymbols() {
  const q = document.getElementById('search').value.toLowerCase();
  document.querySelectorAll('.nav-item').forEach(el => {
    const text = el.textContent.toLowerCase();
    el.style.display = text.includes(q) ? '' : 'none';
  });
  document.querySelectorAll('.doc-item').forEach(el => {
    const text = el.textContent.toLowerCase();
    el.style.display = text.includes(q) ? '' : 'none';
  });
}

function copySnippet(btn) {
  const code = btn.parentElement.querySelector('code').innerText;
  navigator.clipboard.writeText(code).then(() => {
    const orig = btn.innerText;
    btn.innerText = '✓ Copied';
    btn.style.color = '#34d399';
    setTimeout(() => {
      btn.innerText = orig;
      btn.style.color = '';
    }, 2000);
  });
}

// Shortcut: press '/' to focus search
window.addEventListener('keydown', (e) => {
  if (e.key === '/' && document.activeElement.tagName !== 'INPUT') {
    e.preventDefault();
    const s = document.getElementById('search');
    if (s) s.focus();
  }
});
</script>
</body>
</html>
"##,
    );

    html
}

/**
 * Generate a modern, categorized documentation hub / portal (index.html).
 */
pub fn generate_index_html(modules: &[DocModule]) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("<title>Alya Standard Library - Documentation</title>\n");
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str(
        r##"
.hero-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 60px 32px 32px;
  text-align: center;
}

.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  background: rgba(192, 132, 252, 0.1);
  border: 1px solid rgba(192, 132, 252, 0.3);
  border-radius: 9999px;
  color: var(--accent-purple);
  font-size: 0.85rem;
  font-weight: 600;
  margin-bottom: 20px;
}

.hero-title {
  font-size: 3.2rem;
  font-weight: 800;
  letter-spacing: -0.03em;
  line-height: 1.15;
  background: linear-gradient(135deg, #ffffff 30%, #c084fc 70%, #38bdf8 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin-bottom: 16px;
}

.hero-subtitle {
  font-size: 1.2rem;
  color: var(--text-muted);
  max-width: 680px;
  margin: 0 auto 36px;
  line-height: 1.6;
}

.search-hub {
  max-width: 640px;
  margin: 0 auto 48px;
  position: relative;
}

.search-hub input {
  width: 100%;
  padding: 16px 20px 16px 48px;
  background: rgba(13, 19, 35, 0.9);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  color: #fff;
  font-size: 1.05rem;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.3);
  transition: all 0.2s ease;
}

.search-hub input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 4px rgba(56, 189, 248, 0.2);
}

.search-hub .icon {
  position: absolute;
  left: 18px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 1.2rem;
  color: var(--text-dim);
}

.search-hub .shortcut {
  position: absolute;
  right: 16px;
  top: 50%;
  transform: translateY(-50%);
  padding: 3px 8px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--text-dim);
  font-weight: 600;
}

/* Category Filter Tabs */
.filter-tabs {
  display: flex;
  justify-content: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 40px;
}

.tab-btn {
  padding: 8px 16px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-muted);
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-btn:hover, .tab-btn.active {
  background: rgba(56, 189, 248, 0.12);
  color: var(--accent);
  border-color: rgba(56, 189, 248, 0.4);
}

/* Module Grid */
.module-grid {
  max-width: 1200px;
  margin: 0 auto 80px;
  padding: 0 32px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 24px;
}

.module-card {
  background: var(--bg-card);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 24px;
  text-decoration: none;
  color: inherit;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  box-shadow: var(--shadow-card);
  position: relative;
  overflow: hidden;
}

.module-card::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--grad-brand);
  opacity: 0;
  transition: opacity 0.25s ease;
}

.module-card:hover {
  transform: translateY(-4px);
  border-color: var(--border-hover);
  box-shadow: var(--shadow-glow);
}

.module-card:hover::before {
  opacity: 1;
}

.card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.card-cat-badge {
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 3px 8px;
  border-radius: 6px;
}

.card-name {
  font-size: 1.4rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.01em;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-name .arrow {
  color: var(--accent);
  font-size: 1.1rem;
  transition: transform 0.2s ease;
}

.module-card:hover .card-name .arrow {
  transform: translateX(4px);
}

.card-description {
  color: var(--text-muted);
  font-size: 0.92rem;
  line-height: 1.5;
  margin-bottom: 20px;
  flex: 1;
}

.card-stats {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-top: 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
  font-size: 0.8rem;
  color: var(--text-dim);
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.stat-item span.count {
  color: var(--text-muted);
  font-weight: 600;
}

footer {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-dim);
  font-size: 0.85rem;
  border-top: 1px solid var(--border);
}
</style>
</head>
<body>
"##,
    );

    // Top Navigation
    html.push_str("<header class=\"top-nav\">\n");
    html.push_str("  <div class=\"brand-wrapper\">\n");
    html.push_str(ALYA_LOGO_SVG);
    html.push_str("    <span class=\"brand-title\">Alya</span>\n");
    html.push_str("    <span class=\"brand-sub\">Documentation</span>\n");
    html.push_str(&format!(
        "    <span class=\"version-badge\">v{}</span>\n",
        env!("CARGO_PKG_VERSION")
    ));
    html.push_str("  </div>\n");
    html.push_str("  <nav class=\"nav-links\">\n");
    html.push_str(&format!(
        "    <a href=\"https://github.com/alya-lang/alya\" target=\"_blank\" rel=\"noopener noreferrer\" class=\"nav-link github-link\" aria-label=\"GitHub\">\n      {}\n      <span>GitHub</span>\n    </a>\n",
        GITHUB_LOGO_SVG
    ));
    html.push_str("  </nav>\n");
    html.push_str("</header>\n\n");

    // Hero Section
    html.push_str("<div class=\"hero-container\">\n");
    html.push_str(
        "  <div class=\"hero-badge\">⚡ High-Performance Native Standard Library</div>\n",
    );
    html.push_str("  <h1 class=\"hero-title\">Alya Standard Library</h1>\n");
    html.push_str(&format!(
        "  <p class=\"hero-subtitle\">Official API documentation, contracts, and module references across all {} foundational packages.</p>\n",
        modules.len()
    ));

    // Global Search input
    html.push_str("  <div class=\"search-hub\">\n");
    html.push_str("    <span class=\"icon\">🔍</span>\n");
    html.push_str("    <input type=\"text\" id=\"global-search\" placeholder=\"Search modules, functions, and topics...\" oninput=\"filterModules()\">\n");
    html.push_str("    <span class=\"shortcut\">/</span>\n");
    html.push_str("  </div>\n");

    // Filter Chips
    html.push_str("  <div class=\"filter-tabs\">\n");
    html.push_str("    <button class=\"tab-btn active\" onclick=\"filterCategory('all', this)\">All Modules</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('Core', this)\">Core & Math</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('Data', this)\">Data & Collections</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('IO', this)\">I/O & File System</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('System', this)\">System & OS</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('Concurrency', this)\">Concurrency & Net</button>\n");
    html.push_str("    <button class=\"tab-btn\" onclick=\"filterCategory('Tooling', this)\">Testing & Tooling</button>\n");
    html.push_str("  </div>\n");
    html.push_str("</div>\n\n");

    // Modules Grid
    html.push_str("<div class=\"module-grid\" id=\"modules-container\">\n");
    for m in modules {
        let target_file_name = m.name.replace('/', "_");
        let (cat_name, cat_badge_class, cat_key) = categorize_module(&m.name);

        let desc = if !m.description.is_empty() {
            m.description
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .next()
                .unwrap_or(&m.description)
                .to_string()
        } else {
            get_module_fallback_desc(&m.name)
        };

        html.push_str(&format!(
            "  <a href=\"{}.html\" class=\"module-card\" data-category=\"{}\" data-name=\"{}\" data-desc=\"{}\">\n",
            target_file_name,
            cat_key,
            escape_html(&m.name.to_lowercase()),
            escape_html(&desc.to_lowercase())
        ));
        html.push_str("    <div>\n");
        html.push_str("      <div class=\"card-top\">\n");
        html.push_str(&format!(
            "        <span class=\"card-cat-badge {}\">{}</span>\n",
            cat_badge_class, cat_name
        ));
        html.push_str("      </div>\n");
        html.push_str(&format!(
            "      <div class=\"card-name\"><span>std/{}</span><span class=\"arrow\">→</span></div>\n",
            escape_html(&m.name)
        ));
        html.push_str(&format!(
            "      <div class=\"card-description\">{}</div>\n",
            escape_html(&desc)
        ));
        html.push_str("    </div>\n");

        html.push_str("    <div class=\"card-stats\">\n");
        if !m.functions.is_empty() {
            html.push_str(&format!(
                "      <div class=\"stat-item\"><span class=\"count\">{}</span> fn</div>\n",
                m.functions.len()
            ));
        }
        if !m.structs.is_empty() {
            html.push_str(&format!(
                "      <div class=\"stat-item\"><span class=\"count\">{}</span> struct</div>\n",
                m.structs.len()
            ));
        }
        if !m.enums.is_empty() {
            html.push_str(&format!(
                "      <div class=\"stat-item\"><span class=\"count\">{}</span> enum</div>\n",
                m.enums.len()
            ));
        }
        if !m.constants.is_empty() {
            html.push_str(&format!(
                "      <div class=\"stat-item\"><span class=\"count\">{}</span> const</div>\n",
                m.constants.len()
            ));
        }
        html.push_str("    </div>\n");
        html.push_str("  </a>\n");
    }
    html.push_str("</div>\n\n");

    html.push_str("<footer>\n");
    html.push_str("  Generated with <code>alya doc</code> • High-performance native compiler toolchain for Alya.\n");
    html.push_str("</footer>\n\n");

    // Client-side search & category filtering
    html.push_str(
        r##"<script>
let currentCategory = 'all';

function filterCategory(cat, btn) {
  currentCategory = cat;
  document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  applyFilters();
}

function filterModules() {
  applyFilters();
}

function applyFilters() {
  const q = document.getElementById('global-search').value.toLowerCase().trim();
  const cards = document.querySelectorAll('.module-card');
  
  cards.forEach(card => {
    const name = card.dataset.name;
    const desc = card.dataset.desc;
    const cat = card.dataset.category;

    const matchesSearch = !q || name.includes(q) || desc.includes(q);
    const matchesCat = currentCategory === 'all' || cat === currentCategory;

    card.style.display = (matchesSearch && matchesCat) ? 'flex' : 'none';
  });
}

// Press '/' to focus global search
window.addEventListener('keydown', (e) => {
  if (e.key === '/' && document.activeElement.tagName !== 'INPUT') {
    e.preventDefault();
    const s = document.getElementById('global-search');
    if (s) s.focus();
  }
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
 * Renders a code block with syntax highlighting and a floating copy button.
 */
fn render_code_block(sig: &str) -> String {
    let highlighted = highlight_signature(sig);
    format!(
        "<div class=\"code-container\">\n  <button class=\"copy-btn\" onclick=\"copySnippet(this)\">Copy</button>\n  <pre><code>{}</code></pre>\n</div>\n",
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
                out.push_str("<div class=\"code-container\"><button class=\"copy-btn\" onclick=\"copySnippet(this)\">Copy</button><pre><code>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            out.push_str(&escape_html(line));
            out.push('\n');
            continue;
        }

        if trimmed.starts_with("### ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            let title = &trimmed[4..];
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

        if trimmed.starts_with("## ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!(
                "<h3 style=\"font-size: 1.25rem; color: #fff; margin: 20px 0 10px;\">{}</h3>\n",
                escape_html(&trimmed[3..])
            ));
            continue;
        }

        if trimmed.starts_with("# ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!(
                "<h2 style=\"font-size: 1.5rem; color: #fff; margin: 24px 0 12px;\">{}</h2>\n",
                escape_html(&trimmed[2..])
            ));
            continue;
        }

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if !in_list {
                out.push_str("<ul style=\"margin: 8px 0 14px 20px; color: #cbd5e1;\">\n");
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
            "<p style=\"margin-bottom: 12px; color: #cbd5e1; font-size: 0.95rem;\">{}</p>\n",
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
