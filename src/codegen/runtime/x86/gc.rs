use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // alya_gc_color_addr(ptr) -> color address in %eax
    // Single source of truth for cycle-collector color placement
    // (alya-lang/alya#63). Callers guarantee magic 1/2/3 (validated).
    // Colors live in trailing words so every existing header/field offset
    // stays valid: array at handle+16, map at handle+12, struct at
    // handle+4+8*field_count (count from the descriptor). Uses only
    // %eax/%ecx/%edx: preserves %ebx/%esi/%edi across the call.
    // =========================================================================
    out.push_str("alya_gc_color_addr:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str("    je .L_x86_gca_array\n");
    out.push_str("    cmpl $0x5A110002, %ecx\n");
    out.push_str("    je .L_x86_gca_map\n");
    // Struct: uniform 8-byte slots at handle+8*(idx+1); color follows the
    // last field at handle+8+8*field_count (count from the descriptor).
    out.push_str("    movl (%eax), %ecx\n");
    out.push_str("    movl 4(%ecx), %ecx\n");
    out.push_str("    shl $3, %ecx\n");
    out.push_str("    lea 8(%eax, %ecx), %eax\n");
    out.push_str("    jmp .L_x86_gca_done\n");
    out.push_str(".L_x86_gca_array:\n");
    out.push_str("    add $16, %eax\n");
    out.push_str("    jmp .L_x86_gca_done\n");
    out.push_str(".L_x86_gca_map:\n");
    out.push_str("    add $12, %eax\n");
    out.push_str(".L_x86_gca_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_add_purple(ptr)
    // Marks candidate container as PURPLE (color 3) and records it in roots set.
    // =========================================================================
    out.push_str(".global fn_gc_add_purple\n");
    out.push_str("fn_gc_add_purple:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $7, %ebx\n");
    out.push_str("    jnz .L_x86_gc_ap_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_gc_ap_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_gc_ap_done\n");
    out.push_str("    movl -8(%ebx), %eax\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    je .L_x86_gc_ap_valid\n");
    out.push_str("    cmpl $0x5A110002, %eax\n");
    out.push_str("    je .L_x86_gc_ap_valid\n");
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_gc_ap_done\n");
    out.push_str(".L_x86_gc_ap_valid:\n");
    out.push_str("    cmpl $0, alya_gc_in_progress\n");
    out.push_str("    jne .L_x86_gc_ap_done\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmpl $3, (%eax)\n");
    out.push_str("    je .L_x86_gc_ap_done\n");
    out.push_str("    movl $3, (%eax)\n");
    out.push_str("    movl alya_gc_roots_count, %eax\n");
    out.push_str("    cmpl $65536, %eax\n");
    out.push_str("    jae .L_x86_gc_ap_done\n");
    out.push_str("    mov %ebx, alya_gc_roots(, %eax, 4)\n");
    out.push_str("    incl %eax\n");
    out.push_str("    movl %eax, alya_gc_roots_count\n");
    out.push_str("    cmpl $1000000, %eax\n");
    out.push_str("    jb .L_x86_gc_ap_done\n");
    out.push_str("    call fn_gc_collect\n");
    out.push_str(".L_x86_gc_ap_done:\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_mark_gray(s)
    // Trial-decrements reference counts across cyclic subgraph.
    // Frame: locals -16 idx, -20 child.
    // =========================================================================
    out.push_str("alya_gc_mark_gray:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $7, %ebx\n");
    out.push_str("    jnz .L_x86_mg_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_mg_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_mg_done\n");
    out.push_str("    movl -8(%ebx), %eax\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    je .L_x86_mg_valid\n");
    out.push_str("    cmpl $0x5A110002, %eax\n");
    out.push_str("    je .L_x86_mg_valid\n");
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_mg_done\n");
    out.push_str(".L_x86_mg_valid:\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmpl $1, (%eax)\n");
    out.push_str("    je .L_x86_mg_done\n");
    out.push_str("    movl $1, (%eax)\n"); // Color = GREY
    out.push_str("    movl -8(%ebx), %eax\n");

    // Traverse children: Struct (uniform 8-byte slots).
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_mg_arr\n");
    out.push_str("    movl (%ebx), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_mg_done\n");
    out.push_str("    movl 4(%esi), %edi\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_mg_struct_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_mg_done\n");
    out.push_str("    movl 8(%ebx, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_mark_gray(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_mg_struct_loop\n");

    // Traverse children: Array (8-byte slots).
    out.push_str(".L_x86_mg_arr:\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    jne .L_x86_mg_map\n");
    out.push_str("    movl (%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_mg_done\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_mg_done\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_mg_arr_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_mg_done\n");
    out.push_str("    movl (%esi, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_mark_gray(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_mg_arr_loop\n");

    // Traverse children: Map (20-byte entries, kind at +16).
    out.push_str(".L_x86_mg_map:\n");
    out.push_str("    movl 4(%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_mg_done\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_mg_done\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_mg_map_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_mg_done\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    cmpl $1, 16(%eax)\n");
    out.push_str("    jne .L_x86_mg_map_next\n");
    out.push_str("    movl (%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_mark_gray(out);
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    movl 8(%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_mark_gray(out);
    out.push_str(".L_x86_mg_map_next:\n");
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_mg_map_loop\n");

    out.push_str(".L_x86_mg_done:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan(s)
    // Classifies objects into WHITE (garbage) or revives to BLACK if rc > 0.
    // =========================================================================
    out.push_str("alya_gc_scan:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $7, %ebx\n");
    out.push_str("    jnz .L_x86_s_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_s_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_s_done\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %esi\n");
    out.push_str("    cmpl $1, (%esi)\n");
    out.push_str("    jne .L_x86_s_done\n");
    out.push_str("    cmpl $0, -4(%ebx)\n");
    out.push_str("    jle .L_x86_s_to_white\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_scan_black\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_s_done\n");
    out.push_str(".L_x86_s_to_white:\n");
    out.push_str("    movl $2, (%esi)\n"); // Color = WHITE
    out.push_str("    movl -8(%ebx), %eax\n");

    // Struct children scan
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_s_arr\n");
    out.push_str("    movl (%ebx), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_s_done\n");
    out.push_str("    movl 4(%esi), %edi\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_s_struct_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_s_done\n");
    out.push_str("    movl 8(%ebx, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_scan(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_s_struct_loop\n");

    // Array children scan
    out.push_str(".L_x86_s_arr:\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    jne .L_x86_s_map\n");
    out.push_str("    movl (%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_s_done\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_s_done\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_s_arr_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_s_done\n");
    out.push_str("    movl (%esi, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_scan(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_s_arr_loop\n");

    // Map children scan
    out.push_str(".L_x86_s_map:\n");
    out.push_str("    movl 4(%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_s_done\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_s_done\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_s_map_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_s_done\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    cmpl $1, 16(%eax)\n");
    out.push_str("    jne .L_x86_s_map_next\n");
    out.push_str("    movl (%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_scan(out);
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    movl 8(%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_scan(out);
    out.push_str(".L_x86_s_map_next:\n");
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_s_map_loop\n");

    out.push_str(".L_x86_s_done:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan_black(s)
    // Restores trial-decremented reference counts for reachable subgraphs.
    // =========================================================================
    out.push_str("alya_gc_scan_black:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $7, %ebx\n");
    out.push_str("    jnz .L_x86_sb_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_sb_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_sb_done\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movl $0, (%eax)\n"); // Color = BLACK
    out.push_str("    movl -8(%ebx), %eax\n");

    // Struct children scan black
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_sb_arr\n");
    out.push_str("    movl (%ebx), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_sb_done\n");
    out.push_str("    movl 4(%esi), %edi\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_sb_struct_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_sb_done\n");
    out.push_str("    movl 8(%ebx, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_scan_black(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_sb_struct_loop\n");

    // Array children scan black
    out.push_str(".L_x86_sb_arr:\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    jne .L_x86_sb_map\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_sb_done\n");
    out.push_str("    movl (%ebx), %edi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_sb_done\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_sb_arr_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_sb_done\n");
    out.push_str("    movl (%esi, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_scan_black(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_sb_arr_loop\n");

    // Map children scan black (bound is capacity: live entries may sit
    // past len among tombstones).
    out.push_str("    movl 4(%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_sb_done\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_sb_done\n");
    out.push_str(".L_x86_sb_map:\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_sb_map_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_sb_done\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    cmpl $1, 16(%eax)\n");
    out.push_str("    jne .L_x86_sb_map_next\n");
    out.push_str("    movl (%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_scan_black(out);
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    movl 8(%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_scan_black(out);
    out.push_str(".L_x86_sb_map_next:\n");
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_sb_map_loop\n");

    out.push_str(".L_x86_sb_done:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_collect_white(s)
    // Sweeps confirmed WHITE cyclic objects and frees their heap allocations.
    // =========================================================================
    out.push_str("alya_gc_collect_white:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $7, %ebx\n");
    out.push_str("    jnz .L_x86_cw_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_cw_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_cw_done\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %esi\n");
    out.push_str("    cmpl $2, (%esi)\n"); // Must be WHITE (2)
    out.push_str("    jne .L_x86_cw_done\n");
    out.push_str("    movl $0, (%esi)\n"); // Mark BLACK to prevent duplicate sweeping
    out.push_str("    movl -8(%ebx), %eax\n");

    // Struct children collection
    out.push_str("    cmpl $0x5A110003, %eax\n");
    out.push_str("    jne .L_x86_cw_arr\n");
    out.push_str("    movl (%ebx), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_cw_free\n");
    out.push_str("    movl 4(%esi), %edi\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_cw_struct_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_cw_free\n");
    out.push_str("    movl 8(%ebx, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_collect_white(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_cw_struct_loop\n");

    // Array children collection
    out.push_str(".L_x86_cw_arr:\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    jne .L_x86_cw_map\n");
    out.push_str("    movl (%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_cw_free\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_cw_free\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_cw_arr_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_cw_free\n");
    out.push_str("    movl (%esi, %ecx, 8), %eax\n");
    out.push_str("    movl %eax, -20(%ebp)\n");
    emit_child_collect_white(out);
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_cw_arr_loop\n");

    // Map children collection (bound is capacity: live entries may sit
    // past len among tombstones).
    out.push_str(".L_x86_cw_map:\n");
    out.push_str("    cmpl $0x5A110002, %eax\n");
    out.push_str("    jne .L_x86_cw_free\n");
    out.push_str("    movl 4(%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_cw_free\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_cw_free\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str(".L_x86_cw_map_loop:\n");
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    cmp %edi, %ecx\n");
    out.push_str("    jge .L_x86_cw_free\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    cmpl $1, 16(%eax)\n");
    out.push_str("    jne .L_x86_cw_map_next\n");
    out.push_str("    movl (%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_collect_white(out);
    out.push_str("    movl -16(%ebp), %ecx\n");
    out.push_str("    movl %ecx, %eax\n");
    out.push_str("    imul $20, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    movl 8(%eax), %ecx\n");
    out.push_str("    movl %ecx, -20(%ebp)\n");
    emit_child_collect_white(out);
    out.push_str(".L_x86_cw_map_next:\n");
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_cw_map_loop\n");

    out.push_str(".L_x86_cw_free:\n");
    // Free inner data buffers for Array and Map (x86 handle layout:
    // array data at +8/kind at +12, map entries at +8).
    out.push_str("    movl -8(%ebx), %eax\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    je .L_x86_cw_free_inner\n");
    out.push_str("    cmpl $0x5A110002, %eax\n");
    out.push_str("    jne .L_x86_cw_free_outer\n");
    out.push_str(".L_x86_cw_free_inner:\n");
    out.push_str("    movl 8(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_cw_free_outer\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    // Array kind sidecar shares the array free path (maps have none).
    out.push_str("    cmpl $0x5A110001, -8(%ebx)\n");
    out.push_str("    jne .L_x86_cw_free_outer\n");
    out.push_str("    movl 12(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_cw_free_outer\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_cw_free_outer:\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_mem_track_free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    lea -8(%ebx), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    lock incl alya_gc_collected_cycles\n");

    out.push_str(".L_x86_cw_done:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collect() / fn__gc_collect() -> int
    // Executes full Bacon-Rajan Cycle Collection cycle.
    // Returns number of reclaimed cyclic objects.
    // =========================================================================
    out.push_str(".global fn_gc_collect\n");
    out.push_str("fn_gc_collect:\n");
    out.push_str(".global fn__gc_collect\n");
    out.push_str("fn__gc_collect:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");

    // Reentrancy guard
    out.push_str("    cmpl $0, alya_gc_in_progress\n");
    out.push_str("    jne .L_x86_gc_reentrant\n");
    out.push_str("    movl $1, alya_gc_in_progress\n");

    out.push_str("    movl alya_gc_roots_count, %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_gc_early_done\n");

    out.push_str("    movl alya_gc_collected_cycles, %eax\n");
    out.push_str("    movl %eax, -16(%ebp)\n");

    // Phase 1: MarkRoots
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str(".L_x86_gc_p1_loop:\n");
    out.push_str("    movl -20(%ebp), %ecx\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_gc_p1_done\n");
    out.push_str("    movl alya_gc_roots(, %ecx, 4), %ebx\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_gc_p1_next\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %edi\n");
    out.push_str("    cmpl $3, (%edi)\n");
    out.push_str("    je .L_x86_gc_p1_call_mg\n");
    out.push_str("    cmpl $1, (%edi)\n");
    out.push_str("    je .L_x86_gc_p1_next\n");
    out.push_str("    movl $0, (%edi)\n");
    out.push_str("    jmp .L_x86_gc_p1_next\n");
    out.push_str(".L_x86_gc_p1_call_mg:\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_mark_gray\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_gc_p1_next:\n");
    out.push_str("    incl -20(%ebp)\n");
    out.push_str("    jmp .L_x86_gc_p1_loop\n");
    out.push_str(".L_x86_gc_p1_done:\n");

    // Phase 2: ScanRoots
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str(".L_x86_gc_p2_loop:\n");
    out.push_str("    movl -20(%ebp), %ecx\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_gc_p2_done\n");
    out.push_str("    movl alya_gc_roots(, %ecx, 4), %ebx\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_gc_p2_next\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_scan\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_gc_p2_next:\n");
    out.push_str("    incl -20(%ebp)\n");
    out.push_str("    jmp .L_x86_gc_p2_loop\n");
    out.push_str(".L_x86_gc_p2_done:\n");

    // Phase 3: CollectRoots
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str(".L_x86_gc_p3_loop:\n");
    out.push_str("    movl -20(%ebp), %ecx\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_gc_p3_done\n");
    out.push_str("    movl alya_gc_roots(, %ecx, 4), %ebx\n");
    out.push_str("    movl $0, alya_gc_roots(, %ecx, 4)\n"); // Clear slot
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_gc_p3_next\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmpl $2, (%eax)\n");
    out.push_str("    jne .L_x86_gc_p3_next\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_collect_white\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_gc_p3_next:\n");
    out.push_str("    incl -20(%ebp)\n");
    out.push_str("    jmp .L_x86_gc_p3_loop\n");
    out.push_str(".L_x86_gc_p3_done:\n");

    out.push_str("    movl $0, alya_gc_roots_count\n");
    out.push_str("    movl $0, alya_gc_in_progress\n");
    out.push_str("    movl alya_gc_collected_cycles, %eax\n");
    out.push_str("    subl -16(%ebp), %eax\n");
    out.push_str("    jmp .L_x86_gc_exit\n");
    out.push_str(".L_x86_gc_early_done:\n");
    out.push_str("    movl $0, alya_gc_in_progress\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    jmp .L_x86_gc_exit\n");
    out.push_str(".L_x86_gc_reentrant:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_gc_exit:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collected_count() / fn__gc_collected_count() -> int
    // Returns cumulative count of swept cyclic objects.
    // =========================================================================
    out.push_str(".global fn_gc_collected_count\n");
    out.push_str("fn_gc_collected_count:\n");
    out.push_str(".global fn__gc_collected_count\n");
    out.push_str("fn__gc_collected_count:\n");
    out.push_str("    movl alya_gc_collected_cycles, %eax\n");
    out.push_str("    ret\n\n");
}

fn emit_child_mark_gray(out: &mut String) {
    let lbl_next = format!(".L_x86_mg_cn_{:x}", out.len());
    let lbl_cont = format!(".L_x86_mg_cc_{:x}", out.len());
    // Child value staged in -20(%ebp) by the caller.
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    test $7, %eax\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %eax\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str(&format!("    jae {}\n", lbl_next));
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110002, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110003, %ecx\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    lock decl -4(%eax)\n");
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_mark_gray\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan(out: &mut String) {
    let lbl_next = format!(".L_x86_s_cn_{:x}", out.len());
    let lbl_cont = format!(".L_x86_s_cc_{:x}", out.len());
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    test $7, %eax\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %eax\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str(&format!("    jae {}\n", lbl_next));
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110002, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110003, %ecx\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_scan\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan_black(out: &mut String) {
    let lbl_next = format!(".L_x86_sb_cn_{:x}", out.len());
    let lbl_cont = format!(".L_x86_sb_cc_{:x}", out.len());
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    test $7, %eax\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %eax\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str(&format!("    jae {}\n", lbl_next));
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110002, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110003, %ecx\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    lock incl -4(%eax)\n");
    // Recurse only into non-black children (mirrors x64).
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmpl $0, (%eax)\n");
    out.push_str(&format!("    je {}\n", lbl_next));
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_scan_black\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_collect_white(out: &mut String) {
    let lbl_next = format!(".L_x86_cw_cn_{:x}", out.len());
    let lbl_cont = format!(".L_x86_cw_cc_{:x}", out.len());
    let lbl_str = format!(".L_x86_cw_cs_{:x}", out.len());
    let lbl_ext = format!(".L_x86_cw_ce_{:x}", out.len());
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    test $7, %eax\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %eax\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str(&format!("    jae {}\n", lbl_next));
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110002, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110003, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmpl $0x5A110004, %ecx\n");
    out.push_str(&format!("    je {}\n", lbl_str));
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmpl $2, (%eax)\n");
    out.push_str(&format!("    jne {}\n", lbl_ext));
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_gc_collect_white\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_ext));
    out.push_str("    cmpl $0, -4(%eax)\n");
    out.push_str(&format!("    jle {}\n", lbl_next));
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_str));
    out.push_str("    movl -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(&format!("{}:\n", lbl_next));
}
