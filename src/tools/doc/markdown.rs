use super::extractor::DocModule;

/// Demotes raw `### ` headings inside item docs one level (`#### `), so they
/// nest under the page's own `### Item` headings instead of competing with
/// them in the heading tree. Fenced code blocks are left untouched.
fn demote_doc_headings(doc: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && trimmed.starts_with("### ") {
            out.push_str("#### ");
            out.push_str(&trimmed["### ".len()..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

pub fn generate_markdown(module: &DocModule) -> String {
    let mut md = String::new();

    md.push_str(&format!("# Module `{}`\n\n", module.name));
    if !module.description.is_empty() {
        md.push_str(&module.description);
        md.push_str("\n\n");
    }

    // Table of Contents
    md.push_str("## Table of Contents\n\n");
    if !module.constants.is_empty() {
        md.push_str("- [Constants](#constants)\n");
    }
    if !module.interfaces.is_empty() {
        md.push_str("- [Interfaces](#interfaces)\n");
        for iface in &module.interfaces {
            md.push_str(&format!(
                "  - [`{}`](#interface-{})\n",
                iface.name,
                iface.name.to_lowercase()
            ));
        }
    }
    if !module.structs.is_empty() {
        md.push_str("- [Structs](#structs)\n");
        for st in &module.structs {
            md.push_str(&format!(
                "  - [`{}`](#struct-{})\n",
                st.name,
                st.name.to_lowercase()
            ));
        }
    }
    if !module.enums.is_empty() {
        md.push_str("- [Enums](#enums)\n");
        for e in &module.enums {
            md.push_str(&format!(
                "  - [`{}`](#enum-{})\n",
                e.name,
                e.name.to_lowercase()
            ));
        }
    }
    if !module.functions.is_empty() {
        md.push_str("- [Functions](#functions)\n");
        for f in &module.functions {
            md.push_str(&format!(
                "  - [`{}`](#function-{})\n",
                f.name,
                f.name.to_lowercase().replace('.', "-")
            ));
        }
    }
    md.push('\n');

    // Constants
    if !module.constants.is_empty() {
        md.push_str("## Constants\n\n");
        md.push_str("| Name | Value | Visibility | Description |\n");
        md.push_str("|:---|:---|:---|:---|\n");
        for c in &module.constants {
            let vis = if c.is_pub { "`pub`" } else { "private" };
            let doc_preview = c.doc.lines().next().unwrap_or("-");
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                c.name, c.value, vis, doc_preview
            ));
        }
        md.push('\n');
    }

    // Interfaces
    if !module.interfaces.is_empty() {
        md.push_str("## Interfaces\n\n");
        for iface in &module.interfaces {
            md.push_str(&format!("### Interface `{}`\n\n", iface.name));
            if iface.is_pub {
                md.push_str("**Visibility:** `pub`\n\n");
            }
            if !iface.embedded.is_empty() {
                md.push_str(&format!(
                    "**Embedded:** `{}`\n\n",
                    iface.embedded.join("`, `")
                ));
            }
            if !iface.doc.is_empty() {
                md.push_str(&demote_doc_headings(&iface.doc));
                md.push_str("\n\n");
            }

            if !iface.methods.is_empty() {
                md.push_str("#### Methods\n\n");
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
                    md.push_str(&format!(
                        "- `function {}({}) -> {}`\n",
                        m.name,
                        param_str.join(", "),
                        ret
                    ));
                }
                md.push('\n');
            }
        }
    }

    // Structs
    if !module.structs.is_empty() {
        md.push_str("## Structs\n\n");
        for st in &module.structs {
            md.push_str(&format!("### Struct `{}`\n\n", st.name));
            if st.is_pub {
                md.push_str("**Visibility:** `pub`\n\n");
            }
            if !st.doc.is_empty() {
                md.push_str(&demote_doc_headings(&st.doc));
                md.push_str("\n\n");
            }

            if !st.fields.is_empty() {
                md.push_str("#### Fields\n\n");
                md.push_str("| Field | Type | Default | Description |\n");
                md.push_str("|:---|:---|:---|:---|\n");
                for f in &st.fields {
                    let ty = f.type_ann.as_deref().unwrap_or("auto");
                    let def = f.default_val.as_deref().unwrap_or("-");
                    md.push_str(&format!("| `{}` | `{}` | `{}` | - |\n", f.name, ty, def));
                }
                md.push('\n');
            }

            if !st.methods.is_empty() {
                md.push_str("#### Methods\n\n");
                for m in &st.methods {
                    md.push_str(&format!("##### `{}`\n\n", m.signature));
                    if !m.doc.is_empty() {
                        md.push_str(&demote_doc_headings(&m.doc));
                        md.push_str("\n\n");
                    }
                }
            }
        }
    }

    // Enums
    if !module.enums.is_empty() {
        md.push_str("## Enums\n\n");
        for e in &module.enums {
            md.push_str(&format!("### Enum `{}`\n\n", e.name));
            if e.is_pub {
                md.push_str("**Visibility:** `pub`\n\n");
            }
            if !e.doc.is_empty() {
                md.push_str(&demote_doc_headings(&e.doc));
                md.push_str("\n\n");
            }

            md.push_str("| Variant | Explicit Value |\n");
            md.push_str("|:---|:---|\n");
            for v in &e.variants {
                let val = v.value.as_deref().unwrap_or("-");
                md.push_str(&format!("| `{}` | `{}` |\n", v.name, val));
            }
            md.push('\n');
        }
    }

    // Functions
    if !module.functions.is_empty() {
        md.push_str("## Functions\n\n");
        for f in &module.functions {
            md.push_str(&format!("### Function `{}`\n\n", f.name));
            md.push_str("```alya\n");
            md.push_str(&f.signature);
            md.push_str("\n```\n\n");

            if !f.doc.is_empty() {
                md.push_str(&demote_doc_headings(&f.doc));
                md.push_str("\n\n");
            }

            if !f.params.is_empty() {
                md.push_str("**Parameters:**\n\n");
                md.push_str("| Parameter | Type | Default |\n");
                md.push_str("|:---|:---|:---|\n");
                for p in &f.params {
                    let ty = p.type_ann.as_deref().unwrap_or("auto");
                    let def = p.default_val.as_deref().unwrap_or("-");
                    md.push_str(&format!("| `{}` | `{}` | `{}` |\n", p.name, ty, def));
                }
                md.push('\n');
            }

            if let Some(ref ret) = f.return_type {
                md.push_str(&format!("**Returns:** `{}`\n\n", ret));
            }
        }
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demote_doc_headings_nests_under_item() {
        let doc = "Summary.\n\n### Parameters\n- `x`: thing.\n\n### Returns\nValue.\n";
        let out = demote_doc_headings(doc);
        assert!(out.contains("#### Parameters"));
        assert!(out.contains("#### Returns"));
        assert!(!out.lines().any(|l| l.trim_start().starts_with("### ")));
    }

    #[test]
    fn test_demote_doc_headings_preserves_fences_and_deeper_levels() {
        let doc = "```alya\n### not a heading\n```\n\n#### Already deep.\n";
        let out = demote_doc_headings(doc);
        assert!(out.contains("### not a heading"));
        assert!(out.contains("#### Already deep."));
    }
}
