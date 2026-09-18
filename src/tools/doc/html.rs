use super::extractor::DocModule;

pub fn generate_html(module: &DocModule) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!("<title>Module {} - Alya Docs</title>\n", escape_html(&module.name)));
    html.push_str(r#"<style>
:root {
  --bg: #0f172a;
  --bg-sidebar: #1e293b;
  --bg-card: #1e293b;
  --bg-code: #090d16;
  --border: #334155;
  --text: #f8fafc;
  --text-muted: #94a3b8;
  --accent: #38bdf8;
  --accent-badge: #0284c7;
  --pub-badge: #10b981;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background-color: var(--bg);
  color: var(--text);
  line-height: 1.6;
  display: flex;
  min-height: 100vh;
}
aside {
  width: 280px;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  padding: 24px 16px;
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
}
aside h2 {
  font-size: 1.2rem;
  color: var(--accent);
  margin-bottom: 12px;
}
aside input[type="text"] {
  width: 100%;
  padding: 8px 12px;
  background: var(--bg-code);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  margin-bottom: 16px;
}
aside ul { list-style: none; }
aside li { margin: 6px 0; }
aside a {
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.92rem;
  display: block;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.15s ease;
}
aside a:hover {
  background: rgba(56, 189, 248, 0.1);
  color: var(--accent);
}
main {
  flex: 1;
  padding: 40px;
  max-width: 960px;
  overflow-y: auto;
}
h1 { font-size: 2.2rem; margin-bottom: 8px; color: #fff; }
h2 { font-size: 1.5rem; margin: 32px 0 16px; border-bottom: 1px solid var(--border); padding-bottom: 8px; }
h3 { font-size: 1.25rem; margin: 24px 0 12px; color: var(--accent); }
p { margin-bottom: 16px; color: #cbd5e1; }
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 24px;
}
.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  margin-right: 8px;
}
.badge-pub { background: var(--pub-badge); color: #fff; }
.badge-kind { background: var(--border); color: #e2e8f0; }
pre {
  background: var(--bg-code);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 14px 16px;
  overflow-x: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.95rem;
  margin: 12px 0 16px;
  color: #38bdf8;
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: rgba(255, 255, 255, 0.08);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.9em;
}
table {
  width: 100%;
  border-collapse: collapse;
  margin: 16px 0;
}
th, td {
  padding: 10px 14px;
  border: 1px solid var(--border);
  text-align: left;
}
th { background: rgba(0, 0, 0, 0.2); color: var(--accent); font-weight: 600; }
</style>
</head>
<body>
"#);

    // Sidebar
    html.push_str("<aside>\n");
    html.push_str(&format!("<h2>{}</h2>\n", escape_html(&module.name)));
    html.push_str("<input type=\"text\" id=\"search\" placeholder=\"Filter symbols...\" oninput=\"filterSymbols()\">\n");

    if !module.constants.is_empty() {
        html.push_str("<div class=\"nav-group\"><p><strong>Constants</strong></p><ul>\n");
        for c in &module.constants {
            html.push_str(&format!("<li class=\"nav-item\"><a href=\"#c-{}\">{}</a></li>\n", escape_html(&c.name), escape_html(&c.name)));
        }
        html.push_str("</ul></div>\n");
    }

    if !module.interfaces.is_empty() {
        html.push_str("<div class=\"nav-group\"><p><strong>Interfaces</strong></p><ul>\n");
        for iface in &module.interfaces {
            html.push_str(&format!("<li class=\"nav-item\"><a href=\"#if-{}\">{}</a></li>\n", escape_html(&iface.name), escape_html(&iface.name)));
        }
        html.push_str("</ul></div>\n");
    }

    if !module.structs.is_empty() {
        html.push_str("<div class=\"nav-group\"><p><strong>Structs</strong></p><ul>\n");
        for st in &module.structs {
            html.push_str(&format!("<li class=\"nav-item\"><a href=\"#st-{}\">{}</a></li>\n", escape_html(&st.name), escape_html(&st.name)));
        }
        html.push_str("</ul></div>\n");
    }

    if !module.enums.is_empty() {
        html.push_str("<div class=\"nav-group\"><p><strong>Enums</strong></p><ul>\n");
        for e in &module.enums {
            html.push_str(&format!("<li class=\"nav-item\"><a href=\"#en-{}\">{}</a></li>\n", escape_html(&e.name), escape_html(&e.name)));
        }
        html.push_str("</ul></div>\n");
    }

    if !module.functions.is_empty() {
        html.push_str("<div class=\"nav-group\"><p><strong>Functions</strong></p><ul>\n");
        for f in &module.functions {
            html.push_str(&format!("<li class=\"nav-item\"><a href=\"#fn-{}\">{}()</a></li>\n", escape_html(&f.name), escape_html(&f.name)));
        }
        html.push_str("</ul></div>\n");
    }
    html.push_str("</aside>\n");

    // Main content
    html.push_str("<main>\n");
    html.push_str(&format!("<h1>Module <code>{}</code></h1>\n", escape_html(&module.name)));
    if !module.description.is_empty() {
        html.push_str(&render_markdown_html(&module.description));
    }

    // Constants
    if !module.constants.is_empty() {
        html.push_str("<h2>Constants</h2>\n");
        for c in &module.constants {
            html.push_str(&format!("<div class=\"card doc-item\" id=\"c-{}\">\n", escape_html(&c.name)));
            if c.is_pub {
                html.push_str("<span class=\"badge badge-pub\">pub</span>");
            }
            html.push_str("<span class=\"badge badge-kind\">const</span>");
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&c.name)));
            html.push_str(&format!("<pre><code>const {} = {}</code></pre>\n", escape_html(&c.name), escape_html(&c.value)));
            if !c.doc.is_empty() {
                html.push_str(&render_markdown_html(&c.doc));
            }
            html.push_str("</div>\n");
        }
    }

    // Interfaces
    if !module.interfaces.is_empty() {
        html.push_str("<h2>Interfaces</h2>\n");
        for iface in &module.interfaces {
            html.push_str(&format!("<div class=\"card doc-item\" id=\"if-{}\">\n", escape_html(&iface.name)));
            if iface.is_pub {
                html.push_str("<span class=\"badge badge-pub\">pub</span>");
            }
            html.push_str("<span class=\"badge badge-kind\">interface</span>");
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&iface.name)));
            if !iface.doc.is_empty() {
                html.push_str(&render_markdown_html(&iface.doc));
            }
            if !iface.methods.is_empty() {
                html.push_str("<h4>Methods</h4><ul>\n");
                for m in &iface.methods {
                    let param_str: Vec<String> = m.params.iter().map(|p| {
                        if let Some(ref ty) = p.type_ann {
                            format!("{}: {}", p.name, ty)
                        } else {
                            p.name.clone()
                        }
                    }).collect();
                    let ret = m.return_type.as_deref().unwrap_or("void");
                    html.push_str(&format!("<li><code>function {}({}) -> {}</code></li>\n", escape_html(&m.name), escape_html(&param_str.join(", ")), escape_html(ret)));
                }
                html.push_str("</ul>\n");
            }
            html.push_str("</div>\n");
        }
    }

    // Structs
    if !module.structs.is_empty() {
        html.push_str("<h2>Structs</h2>\n");
        for st in &module.structs {
            html.push_str(&format!("<div class=\"card doc-item\" id=\"st-{}\">\n", escape_html(&st.name)));
            if st.is_pub {
                html.push_str("<span class=\"badge badge-pub\">pub</span>");
            }
            html.push_str("<span class=\"badge badge-kind\">struct</span>");
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&st.name)));
            if !st.doc.is_empty() {
                html.push_str(&render_markdown_html(&st.doc));
            }
            if !st.fields.is_empty() {
                html.push_str("<h4>Fields</h4><table>\n<thead><tr><th>Field</th><th>Type</th><th>Default</th></tr></thead>\n<tbody>\n");
                for f in &st.fields {
                    let ty = f.type_ann.as_deref().unwrap_or("auto");
                    let def = f.default_val.as_deref().unwrap_or("-");
                    html.push_str(&format!("<tr><td><code>{}</code></td><td><code>{}</code></td><td><code>{}</code></td></tr>\n", escape_html(&f.name), escape_html(ty), escape_html(def)));
                }
                html.push_str("</tbody></table>\n");
            }
            if !st.methods.is_empty() {
                html.push_str("<h4>Methods</h4>\n");
                for m in &st.methods {
                    html.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(&m.signature)));
                    if !m.doc.is_empty() {
                        html.push_str(&render_markdown_html(&m.doc));
                    }
                }
            }
            html.push_str("</div>\n");
        }
    }

    // Enums
    if !module.enums.is_empty() {
        html.push_str("<h2>Enums</h2>\n");
        for e in &module.enums {
            html.push_str(&format!("<div class=\"card doc-item\" id=\"en-{}\">\n", escape_html(&e.name)));
            if e.is_pub {
                html.push_str("<span class=\"badge badge-pub\">pub</span>");
            }
            html.push_str("<span class=\"badge badge-kind\">enum</span>");
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&e.name)));
            if !e.doc.is_empty() {
                html.push_str(&render_markdown_html(&e.doc));
            }
            html.push_str("<table>\n<thead><tr><th>Variant</th><th>Value</th></tr></thead>\n<tbody>\n");
            for v in &e.variants {
                let val = v.value.as_deref().unwrap_or("-");
                html.push_str(&format!("<tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n", escape_html(&v.name), escape_html(val)));
            }
            html.push_str("</tbody></table>\n");
            html.push_str("</div>\n");
        }
    }

    // Functions
    if !module.functions.is_empty() {
        html.push_str("<h2>Functions</h2>\n");
        for f in &module.functions {
            html.push_str(&format!("<div class=\"card doc-item\" id=\"fn-{}\">\n", escape_html(&f.name)));
            if f.is_pub {
                html.push_str("<span class=\"badge badge-pub\">pub</span>");
            }
            html.push_str("<span class=\"badge badge-kind\">function</span>");
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&f.name)));
            html.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(&f.signature)));
            if !f.doc.is_empty() {
                html.push_str(&render_markdown_html(&f.doc));
            }
            if !f.params.is_empty() {
                html.push_str("<h4>Parameters</h4><table>\n<thead><tr><th>Parameter</th><th>Type</th><th>Default</th></tr></thead>\n<tbody>\n");
                for p in &f.params {
                    let ty = p.type_ann.as_deref().unwrap_or("auto");
                    let def = p.default_val.as_deref().unwrap_or("-");
                    html.push_str(&format!("<tr><td><code>{}</code></td><td><code>{}</code></td><td><code>{}</code></td></tr>\n", escape_html(&p.name), escape_html(ty), escape_html(def)));
                }
                html.push_str("</tbody></table>\n");
            }
            if let Some(ref ret) = f.return_type {
                html.push_str(&format!("<p><strong>Returns:</strong> <code>{}</code></p>\n", escape_html(ret)));
            }
            html.push_str("</div>\n");
        }
    }

    html.push_str("</main>\n");

    // Filter script
    html.push_str(r#"<script>
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
</script>
</body>
</html>
"#);

    html
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

fn render_markdown_html(md: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                out.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                out.push_str("<pre><code>");
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
            out.push_str(&format!("<h4>{}</h4>\n", escape_html(&trimmed[4..])));
        } else if trimmed.starts_with("## ") {
            out.push_str(&format!("<h3>{}</h3>\n", escape_html(&trimmed[3..])));
        } else if trimmed.starts_with("# ") {
            out.push_str(&format!("<h2>{}</h2>\n", escape_html(&trimmed[2..])));
        } else if trimmed.is_empty() {
            out.push_str("<br>\n");
        } else {
            out.push_str(&format!("<p>{}</p>\n", escape_html(line)));
        }
    }

    if in_code_block {
        out.push_str("</code></pre>\n");
    }

    out
}
