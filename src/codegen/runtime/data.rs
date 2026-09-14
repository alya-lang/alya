use crate::codegen::context::StructDefInfo;
use crate::codegen::target::{Architecture, OperatingSystem};
use std::collections::HashMap;

pub fn emit_data_sections(
    out: &mut String,
    arch: Architecture,
    os: OperatingSystem,
    structs: &HashMap<String, StructDefInfo>,
) {
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("\n.section __DATA,__bss\n");
        out.push_str(".p2align 4\n");
    } else {
        out.push_str("\n.section .bss\n");
        out.push_str(".align 16\n");
    }

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
        out.push_str(&format!("    {} \"%ld\"\n", str_directive));
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
        out.push_str(&format!("    {} \"%s: %ld\"\n", str_directive));
    }
    out.push_str("alya_fmt_struct_comma:\n");
    out.push_str(&format!("    {} \", \"\n", str_directive));

    // Struct name and field name strings
    for (name, sdef) in structs {
        let name_label = format!("alya_struct_{}_name", name);
        out.push_str(&format!(
            "{}:\n    {} \"{}\"\n",
            name_label, str_directive, name
        ));
        for (i, f) in sdef.fields.iter().enumerate() {
            let field_label = format!("alya_struct_{}_f_{}", name, i);
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

    for (name, sdef) in structs {
        let desc_label = format!("alya_struct_desc_{}", name);
        out.push_str(&format!("{}:\n", desc_label));
        out.push_str(&format!("    {} alya_struct_{}_name\n", ptr_dir, name));
        out.push_str(&format!("    {} {}\n", ptr_dir, sdef.fields.len()));
        for (i, _) in sdef.fields.iter().enumerate() {
            out.push_str(&format!("    {} alya_struct_{}_f_{}\n", ptr_dir, name, i));
        }
    }

    out.push_str(".text\n");
}
