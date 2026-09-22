use crate::codegen::context::StructDefInfo;
use crate::codegen::target::{Architecture, OperatingSystem};
use std::collections::HashMap;

pub fn emit_data_sections(
    out: &mut String,
    arch: Architecture,
    os: OperatingSystem,
    structs: &HashMap<String, StructDefInfo>,
    interfaces: &HashMap<String, crate::codegen::context::InterfaceDefInfo>,
    vtables: &HashMap<(String, String), String>,
    functions: &std::collections::HashSet<String>,
) {
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("\n.section __DATA,__bss\n");
        out.push_str(".p2align 4\n");
    } else {
        out.push_str("\n.section .bss\n");
        out.push_str(".align 16\n");
    }

    out.push_str("alya_str_buf_guard:\n");
    out.push_str("    .space 64\n");
    out.push_str("alya_str_buf:\n");
    out.push_str("    .space 67108864\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("alya_wsa_data:\n");
        out.push_str("    .space 512\n");
    }
    match arch {
        Architecture::ARM64 | Architecture::X64 => {
            out.push_str("alya_str_idx:\n");
            out.push_str("    .space 512\n");
            out.push_str("alya_thread_slot_seq:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_argc:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_argv:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_catch_idx:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_catch_stack_handler:\n");
            out.push_str("    .space 1024\n");
            out.push_str("alya_catch_stack_sp:\n");
            out.push_str("    .space 1024\n");
            out.push_str("alya_catch_stack_bp:\n");
            out.push_str("    .space 1024\n");
            out.push_str("alya_err_msg:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_rand_state:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_allocated_bytes:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_trace_enabled:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_report_done:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_records_head:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_total_allocs:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_total_frees:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_active_allocs:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_total_bytes:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_active_bytes:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_peak_bytes:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_live_structs:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_live_arrays:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_live_maps:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_live_strings:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_mem_live_raw:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_gc_roots:\n");
            out.push_str("    .space 524288\n");
            out.push_str("alya_gc_roots_count:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_gc_collected_cycles:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_gc_in_progress:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_count:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_active_count:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_runqueue_head:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_runqueue_tail:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_current:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_sched_ctx:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_main:\n");
            out.push_str("    .space 128\n");
            out.push_str("alya_fiber_lock:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_seq:\n");
            out.push_str("    .quad 0\n");
            out.push_str("alya_fiber_pool_head:\n");
            out.push_str("    .quad 0\n");
        }
        Architecture::X86 => {
            out.push_str("alya_str_idx:\n");
            out.push_str("    .space 256\n");
            out.push_str("alya_thread_slot_seq:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_argc:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_argv:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_catch_idx:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_catch_stack_handler:\n");
            out.push_str("    .space 512\n");
            out.push_str("alya_catch_stack_sp:\n");
            out.push_str("    .space 512\n");
            out.push_str("alya_catch_stack_bp:\n");
            out.push_str("    .space 512\n");
            out.push_str("alya_err_msg:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_rand_state:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_allocated_bytes:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_trace_enabled:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_report_done:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_records_head:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_total_allocs:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_total_frees:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_active_allocs:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_total_bytes:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_active_bytes:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_peak_bytes:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_live_structs:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_live_arrays:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_live_maps:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_live_strings:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_mem_live_raw:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_gc_roots:\n");
            out.push_str("    .space 262144\n");
            out.push_str("alya_gc_roots_count:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_gc_collected_cycles:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_gc_in_progress:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_count:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_active_count:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_runqueue_head:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_runqueue_tail:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_current:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_sched_ctx:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_main:\n");
            out.push_str("    .space 64\n");
            out.push_str("alya_fiber_lock:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_seq:\n");
            out.push_str("    .long 0\n");
            out.push_str("alya_fiber_pool_head:\n");
            out.push_str("    .long 0\n");
        }
    }

    let is_macos = matches!(os, OperatingSystem::MacOS);
    let str_directive = if is_macos { ".asciz" } else { ".string" };

    if is_macos {
        out.push_str("\n.section __TEXT,__cstring,cstring_literals\n");
    } else {
        out.push_str("\n.section .rodata\n");
    }

    let os_str = match os {
        OperatingSystem::Windows => "windows",
        OperatingSystem::Linux => "linux",
        OperatingSystem::MacOS => "macos",
    };
    let arch_str = match arch {
        Architecture::X64 => "x64",
        Architecture::X86 => "x86",
        Architecture::ARM64 => "arm64",
    };
    out.push_str("alya_str_target_os:\n");
    out.push_str(&format!("    {} \"{}\"\n", str_directive, os_str));
    out.push_str("alya_str_target_arch:\n");
    out.push_str(&format!("    {} \"{}\"\n", str_directive, arch_str));
    out.push_str("alya_str_empty:\n");
    out.push_str(&format!("    {} \"\"\n", str_directive));
    out.push_str("alya_str_mode_rb:\n");
    out.push_str(&format!("    {} \"rb\"\n", str_directive));
    out.push_str("alya_str_mode_wb:\n");
    out.push_str(&format!("    {} \"wb\"\n", str_directive));
    out.push_str("alya_str_mode_ab:\n");
    out.push_str(&format!("    {} \"ab\"\n", str_directive));
    out.push_str("alya_fmt_prompt:\n");
    out.push_str(&format!("    {} \"%s\"\n", str_directive));
    out.push_str("alya_fmt_say_str:\n");
    out.push_str(&format!("    {} \"%s\\n\"\n", str_directive));
    out.push_str("alya_fmt_flt_val:\n");
    out.push_str(&format!("    {} \"%g\"\n", str_directive));
    out.push_str("alya_str_console_clear:\n");
    out.push_str(
        "    .byte 0x1b, 0x5b, 0x32, 0x4a, 0x1b, 0x5b, 0x33, 0x4a, 0x1b, 0x5b, 0x48, 0x00\n",
    );
    out.push_str("alya_fmt_console_title:\n");
    out.push_str("    .byte 0x1b, 0x5d, 0x30, 0x3b, 0x25, 0x73, 0x07, 0x00\n");
    out.push_str("alya_str_console_bell:\n");
    out.push_str("    .byte 0x07, 0x00\n");
    out.push_str("alya_fmt_runtime_err:\n");
    out.push_str(&format!("    {} \"Runtime error: %s\\n\"\n", str_directive));
    out.push_str("alya_str_unhandled_err:\n");
    out.push_str(&format!("    {} \"unhandled error\"\n", str_directive));
    out.push_str("alya_fmt_div_zero:\n");
    out.push_str(&format!(
        "    {} \"Runtime error: division by zero\\n\"\n",
        str_directive
    ));
    out.push_str("alya_str_div_zero:\n");
    out.push_str(&format!("    {} \"division by zero\"\n", str_directive));
    out.push_str("alya_fmt_bounds:\n");
    out.push_str(&format!(
        "    {} \"Runtime error: index out of bounds\\n\"\n",
        str_directive
    ));
    out.push_str("alya_str_bounds:\n");
    out.push_str(&format!("    {} \"index out of bounds\"\n", str_directive));
    out.push_str("alya_str_null_unwrap:\n");
    out.push_str(&format!("    {} \"force unwrap of null\"\n", str_directive));
    out.push_str("alya_fmt_arr_empty:\n");
    out.push_str(&format!("    {} \"[]\\n\"\n", str_directive));
    out.push_str("alya_fmt_arr_open:\n");
    out.push_str(&format!("    {} \"[\"\n", str_directive));
    out.push_str("alya_fmt_arr_close:\n");
    out.push_str(&format!("    {} \"]\\n\"\n", str_directive));
    out.push_str("alya_fmt_arr_elem:\n");
    if matches!(arch, Architecture::X86) {
        out.push_str(&format!("    {} \"%d\"\n", str_directive));
    } else {
        out.push_str(&format!("    {} \"%lld\"\n", str_directive));
    }
    out.push_str("alya_fmt_arr_comma:\n");
    out.push_str(&format!("    {} \", \"\n", str_directive));

    // Map format strings
    out.push_str("alya_fmt_map_null:\n");
    out.push_str(&format!("    {} \"null\\n\"\n", str_directive));
    out.push_str("alya_fmt_map_empty:\n");
    out.push_str(&format!("    {} \"{{}}\\n\"\n", str_directive));
    out.push_str("alya_fmt_map_open:\n");
    out.push_str(&format!("    {} \"{{\"\n", str_directive));
    out.push_str("alya_fmt_map_close:\n");
    out.push_str(&format!("    {} \"}}\\n\"\n", str_directive));
    out.push_str("alya_fmt_map_colon:\n");
    out.push_str(&format!("    {} \": \"\n", str_directive));

    // Struct format strings
    out.push_str("alya_fmt_struct_null:\n");
    out.push_str(&format!("    {} \"null\\n\"\n", str_directive));
    out.push_str("alya_fmt_struct_open:\n");
    out.push_str(&format!("    {} \"%s {{ \"\n", str_directive));
    out.push_str("alya_fmt_struct_close:\n");
    out.push_str(&format!("    {} \" }}\\n\"\n", str_directive));
    out.push_str("alya_fmt_struct_field:\n");
    if matches!(arch, Architecture::X86) {
        out.push_str(&format!("    {} \"%s: %d\"\n", str_directive));
    } else {
        out.push_str(&format!("    {} \"%s: %lld\"\n", str_directive));
    }

    out.push_str("alya_fmt_struct_comma:\n");
    out.push_str(&format!("    {} \", \"\n", str_directive));

    // Memory trace report format strings
    let int_fmt = if matches!(arch, Architecture::X86) {
        "%d"
    } else {
        "%lld"
    };
    out.push_str("alya_mem_fmt_header:\n");
    out.push_str(&format!("    {} \"\\n============================================================\\n                 ALYA MEMORY TRACE & LEAK REPORT\\n============================================================\\n\"\n", str_directive));
    out.push_str("alya_mem_fmt_totals:\n");
    out.push_str(&format!("    {} \"  Total Allocations   : {}\\n  Total Deallocations : {}\\n  Active Allocations  : {}\\n\"\n", str_directive, int_fmt, int_fmt, int_fmt));
    out.push_str("alya_mem_fmt_bytes:\n");
    out.push_str(&format!("    {} \"  Total Allocated     : {} bytes\\n  Peak Memory Usage   : {} bytes\\n  Active Heap Memory  : {} bytes\\n\\n\"\n", str_directive, int_fmt, int_fmt, int_fmt));
    out.push_str("alya_mem_fmt_objects:\n");
    out.push_str(&format!("    {} \"  Live Object Summary:\\n    * Structs         : {} active\\n    * Arrays          : {} active\\n    * Maps            : {} active\\n    * Strings         : {} active\\n    * Raw Buffers     : {} active\\n\"\n", str_directive, int_fmt, int_fmt, int_fmt, int_fmt, int_fmt));
    out.push_str("alya_mem_fmt_clean:\n");
    out.push_str(&format!("    {} \"------------------------------------------------------------\\n  STATUS: [OK] Clean execution, 0 memory leaks detected\\n============================================================\\n\\n\"\n", str_directive));
    out.push_str("alya_mem_fmt_warn:\n");
    out.push_str(&format!("    {} \"------------------------------------------------------------\\n  STATUS: [WARN] {} memory leak(s) detected ({} bytes uncollected)\\n\\n  Uncollected Allocations (Leak Attribution):\\n\"\n", str_directive, int_fmt, int_fmt));
    out.push_str("alya_mem_fmt_item_struct:\n");
    out.push_str(&format!(
        "    {} \"    [{}] %p ({} bytes) Type: Struct (%s)\\n\"\n",
        str_directive, int_fmt, int_fmt
    ));
    out.push_str("alya_mem_fmt_item_array:\n");
    out.push_str(&format!(
        "    {} \"    [{}] %p ({} bytes) Type: Array\\n\"\n",
        str_directive, int_fmt, int_fmt
    ));
    out.push_str("alya_mem_fmt_item_map:\n");
    out.push_str(&format!(
        "    {} \"    [{}] %p ({} bytes) Type: Map\\n\"\n",
        str_directive, int_fmt, int_fmt
    ));
    out.push_str("alya_mem_fmt_item_str:\n");
    out.push_str(&format!(
        "    {} \"    [{}] %p ({} bytes) Type: String\\n\"\n",
        str_directive, int_fmt, int_fmt
    ));
    out.push_str("alya_mem_fmt_item_raw:\n");
    out.push_str(&format!(
        "    {} \"    [{}] %p ({} bytes) Type: Raw Buffer\\n\"\n",
        str_directive, int_fmt, int_fmt
    ));
    out.push_str("alya_mem_fmt_footer:\n");
    out.push_str(&format!(
        "    {} \"============================================================\\n\\n\"\n",
        str_directive
    ));
    out.push_str("alya_str_anon_struct:\n");
    out.push_str(&format!("    {} \"Anonymous\"\n", str_directive));

    // Struct name and field name strings
    let mut emitted_names = std::collections::HashSet::new();
    for (name, sdef) in structs {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        if !emitted_names.insert(bare.to_string()) {
            continue;
        }
        let name_label = format!("alya_struct_{}_name", bare);
        out.push_str(&format!(
            "{}:\n    {} \"{}\"\n",
            name_label, str_directive, bare
        ));
        for (i, f) in sdef.fields.iter().enumerate() {
            let field_label = format!("alya_struct_{}_f_{}", bare, i);
            out.push_str(&format!(
                "{}:\n    {} \"{}\"\n",
                field_label, str_directive, f
            ));
        }
    }

    out.push_str(".global alya_rodata_end\n");
    out.push_str("alya_rodata_end:\n");
    out.push_str("    .byte 0\n");

    // Struct descriptors in data section
    if is_macos {
        out.push_str("\n.section __DATA,__data\n");
    } else {
        out.push_str("\n.section .data\n");
    }

    let ptr_dir = if matches!(arch, Architecture::X86) {
        ".long"
    } else {
        ".quad"
    };

    let mut emitted_descs = std::collections::HashSet::new();
    for (name, sdef) in structs {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let desc_label = format!("alya_struct_desc_{}", bare);
        if !emitted_descs.insert(desc_label.clone()) {
            continue;
        }
        out.push_str(&format!("{}:\n", desc_label));
        out.push_str(&format!("    {} alya_struct_{}_name\n", ptr_dir, bare));
        out.push_str(&format!("    {} {}\n", ptr_dir, sdef.fields.len()));
        for (i, _) in sdef.fields.iter().enumerate() {
            out.push_str(&format!("    {} alya_struct_{}_f_{}\n", ptr_dir, bare, i));
        }
    }

    let mut emitted_vtables = std::collections::HashSet::new();
    for ((sname, iname), vtable_label) in vtables {
        if !emitted_vtables.insert(vtable_label.clone()) {
            continue;
        }
        let bare_s = sname.rsplit("::").next().unwrap_or(sname);
        let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
        let flattened = crate::codegen::get_interface_flattened_methods(iname, interfaces);
        out.push_str(&format!(".global {}\n", vtable_label));
        out.push_str(&format!("{}:\n", vtable_label));
        out.push_str(&format!("    {} alya_struct_desc_{}\n", ptr_dir, bare_s));
        for m in &flattened {
            let fn_target = if functions.contains(&format!("{}__{}", sname, m.name)) {
                format!("{}__{}", sname, m.name)
            } else if functions.contains(&format!("{}__{}", bare_s, m.name)) {
                format!("{}__{}", bare_s, m.name)
            } else if let Some(matching) = functions.iter().find(|f| {
                (f.ends_with(&format!("__{}", m.name)) || f.ends_with(&format!("::{}", m.name)))
                    && f.contains(bare_s)
            }) {
                matching.clone()
            } else {
                format!("{}__{}", bare_s, m.name)
            };
            let mangled = crate::codegen::arch::control::mangle_symbol_name(&fn_target);
            out.push_str(&format!("    {} fn_{}\n", ptr_dir, mangled));
        }
    }

    out.push_str(".text\n");
}
