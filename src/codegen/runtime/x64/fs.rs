use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let (opendir_fn, readdir_fn, closedir_fn) = if matches!(os, OperatingSystem::MacOS) {
        ("_opendir$INODE64", "_readdir$INODE64", "_closedir")
    } else {
        ("opendir", "readdir", "closedir")
    };
    let _ = (is_win, p);

    // fn_file_exists
    out.push_str("fn_file_exists:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_fexists_no\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rdx\n");
        out.push_str("    call fopen\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rsi\n");
        out.push_str(&format!("    call {}fopen\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fexists_no\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call fclose\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}fclose\n", p));
    }
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_fexists_end\n");
    out.push_str(".L_x64_fexists_no:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fexists_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_delete_file / fn_remove_file
    out.push_str(".global fn_delete_file\n");
    out.push_str("fn_delete_file:\n");
    out.push_str(".global fn_remove_file\n");
    out.push_str("fn_remove_file:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_fdel_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call remove\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}remove\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_fdel_fail\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_fdel_end\n");
    out.push_str(".L_x64_fdel_fail:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fdel_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_write_file
    out.push_str("fn_write_file:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_fwrite_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    lea alya_str_mode_wb(%rip), %rdx\n");
        out.push_str("    call fopen\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    lea alya_str_mode_wb(%rip), %rsi\n");
        out.push_str(&format!("    call {}fopen\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fwrite_fail\n");
    out.push_str("    mov %rax, %r14\n"); // r14 = fp
    out.push_str("    xor %r15, %r15\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_fwrite_do_write\n");
    out.push_str(".L_x64_fwrite_len_loop:\n");
    out.push_str("    cmpb $0, (%r13, %r15)\n");
    out.push_str("    je .L_x64_fwrite_do_write\n");
    out.push_str("    inc %r15\n");
    out.push_str("    jmp .L_x64_fwrite_len_loop\n");
    out.push_str(".L_x64_fwrite_do_write:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    mov $1, %rdx\n");
        out.push_str("    mov %r15, %r8\n");
        out.push_str("    mov %r14, %r9\n");
        out.push_str("    call fwrite\n");
        out.push_str("    mov %r14, %rcx\n");
        out.push_str("    call fclose\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    mov $1, %rsi\n");
        out.push_str("    mov %r15, %rdx\n");
        out.push_str("    mov %r14, %rcx\n");
        out.push_str(&format!("    call {}fwrite\n", p));
        out.push_str("    mov %r14, %rdi\n");
        out.push_str(&format!("    call {}fclose\n", p));
    }
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_fwrite_end\n");
    out.push_str(".L_x64_fwrite_fail:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fwrite_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_append_file
    out.push_str(".global fn_append_file\n");
    out.push_str("fn_append_file:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_fapp_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    lea alya_str_mode_ab(%rip), %rdx\n");
        out.push_str("    call fopen\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    lea alya_str_mode_ab(%rip), %rsi\n");
        out.push_str(&format!("    call {}fopen\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fapp_fail\n");
    out.push_str("    mov %rax, %r14\n");
    out.push_str("    xor %r15, %r15\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_fapp_do_write\n");
    out.push_str(".L_x64_fapp_len_loop:\n");
    out.push_str("    cmpb $0, (%r13, %r15)\n");
    out.push_str("    je .L_x64_fapp_do_write\n");
    out.push_str("    inc %r15\n");
    out.push_str("    jmp .L_x64_fapp_len_loop\n");
    out.push_str(".L_x64_fapp_do_write:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    mov $1, %rdx\n");
        out.push_str("    mov %r15, %r8\n");
        out.push_str("    mov %r14, %r9\n");
        out.push_str("    call fwrite\n");
        out.push_str("    mov %r14, %rcx\n");
        out.push_str("    call fclose\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    mov $1, %rsi\n");
        out.push_str("    mov %r15, %rdx\n");
        out.push_str("    mov %r14, %rcx\n");
        out.push_str(&format!("    call {}fwrite\n", p));
        out.push_str("    mov %r14, %rdi\n");
        out.push_str(&format!("    call {}fclose\n", p));
    }
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_fapp_end\n");
    out.push_str(".L_x64_fapp_fail:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fapp_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_read_file
    out.push_str("fn_read_file:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %r12\n");
    }
    out.push_str("    lea alya_str_empty(%rip), %rbx\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_fread_ret\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rdx\n");
        out.push_str("    call fopen\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rsi\n");
        out.push_str(&format!("    call {}fopen\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fread_ret\n");
    out.push_str("    mov %rax, %r12\n"); // r12 = fp
                                          // fseek(fp, 0, 2)
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    mov $2, %r8\n");
        out.push_str("    call fseek\n");
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call ftell\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str("    mov $2, %rdx\n");
        out.push_str(&format!("    call {}fseek\n", p));
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}ftell\n", p));
    }
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jl .L_x64_fread_close\n");
    out.push_str("    mov %rax, %r13\n"); // r13 = len
                                          // fseek(fp, 0, 0)
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    xor %r8, %r8\n");
        out.push_str("    call fseek\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    lea 1(%r13), %rdx\n");
        out.push_str("    call calloc\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str(&format!("    call {}fseek\n", p));
        out.push_str("    mov $1, %rdi\n");
        out.push_str("    lea 1(%r13), %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    mov %rax, %r14\n"); // r14 = buf
    out.push_str("    mov %rax, %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r14, %rcx\n");
        out.push_str("    mov $1, %rdx\n");
        out.push_str("    mov %r13, %r8\n");
        out.push_str("    mov %r12, %r9\n");
        out.push_str("    call fread\n");
    } else {
        out.push_str("    mov %r14, %rdi\n");
        out.push_str("    mov $1, %rsi\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    mov %r12, %rcx\n");
        out.push_str(&format!("    call {}fread\n", p));
    }
    out.push_str("    movb $0, (%r14, %r13)\n");
    out.push_str(".L_x64_fread_close:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call fclose\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}fclose\n", p));
    }
    out.push_str(".L_x64_fread_ret:\n");
    out.push_str("    mov %rbx, %rax\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_file_size
    out.push_str(".global fn_file_size\n");
    out.push_str("fn_file_size:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    sub $16, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_fsize_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rdx\n");
        out.push_str("    call fopen\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    lea alya_str_mode_rb(%rip), %rsi\n");
        out.push_str(&format!("    call {}fopen\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fsize_fail\n");
    out.push_str("    mov %rax, %r12\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    mov $2, %r8\n");
        out.push_str("    call fseek\n");
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call ftell\n");
        out.push_str("    mov %rax, %rbx\n");
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call fclose\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str("    mov $2, %rdx\n");
        out.push_str(&format!("    call {}fseek\n", p));
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}ftell\n", p));
        out.push_str("    mov %rax, %rbx\n");
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}fclose\n", p));
    }
    out.push_str("    mov %rbx, %rax\n");
    out.push_str("    jmp .L_x64_fsize_end\n");
    out.push_str(".L_x64_fsize_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_fsize_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    add $16, %rsp\n");
    }
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_make_dir / fn_mkdir
    out.push_str(".global fn_make_dir\n");
    out.push_str("fn_make_dir:\n");
    out.push_str(".global fn_mkdir\n");
    out.push_str("fn_mkdir:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_mkdir_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call _mkdir\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov $511, %rsi\n");
        out.push_str(&format!("    call {}mkdir\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_mkdir_fail\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_mkdir_end\n");
    out.push_str(".L_x64_mkdir_fail:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_mkdir_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_remove_dir / fn_rmdir
    out.push_str(".global fn_remove_dir\n");
    out.push_str("fn_remove_dir:\n");
    out.push_str(".global fn_rmdir\n");
    out.push_str("fn_rmdir:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_rmdir_fail\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call _rmdir\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}rmdir\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_rmdir_fail\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_rmdir_end\n");
    out.push_str(".L_x64_rmdir_fail:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_rmdir_end:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_is_dir
    out.push_str(".global fn_is_dir\n");
    out.push_str("fn_is_dir:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_isdir_no\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call GetFileAttributesA\n");
        out.push_str("    cmp $-1, %eax\n");
        out.push_str("    je .L_x64_isdir_no\n");
        out.push_str("    test $0x10, %eax\n");
        out.push_str("    jz .L_x64_isdir_no\n");
        out.push_str("    mov $1, %rax\n");
        out.push_str("    jmp .L_x64_isdir_end\n");
        out.push_str(".L_x64_isdir_no:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_isdir_end:\n");
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_isdir_no\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}\n", opendir_fn));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_isdir_no\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}\n", closedir_fn));
        out.push_str("    mov $1, %rax\n");
        out.push_str("    jmp .L_x64_isdir_end\n");
        out.push_str(".L_x64_isdir_no:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_isdir_end:\n");
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_list_dir / fn_read_dir
    out.push_str(".global fn_list_dir\n");
    out.push_str("fn_list_dir:\n");
    out.push_str(".global fn_read_dir\n");
    out.push_str("fn_read_dir:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if is_win {
        out.push_str("    sub $888, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_array_new\n");
        out.push_str("    mov %rax, %r12\n");
        out.push_str("    lea 352(%rsp), %rdi\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_ld_win_dot\n");
        out.push_str("    cmpb $0, (%rbx)\n");
        out.push_str("    je .L_x64_ld_win_dot\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str(".L_x64_ld_win_cp:\n");
        out.push_str("    movb (%rsi), %al\n");
        out.push_str("    test %al, %al\n");
        out.push_str("    jz .L_x64_ld_win_cp_done\n");
        out.push_str("    movb %al, (%rdi)\n");
        out.push_str("    inc %rsi\n");
        out.push_str("    inc %rdi\n");
        out.push_str("    jmp .L_x64_ld_win_cp\n");
        out.push_str(".L_x64_ld_win_dot:\n");
        out.push_str("    movb $46, (%rdi)\n");
        out.push_str("    inc %rdi\n");
        out.push_str("    jmp .L_x64_ld_win_star\n");
        out.push_str(".L_x64_ld_win_cp_done:\n");
        out.push_str("    movb -1(%rdi), %al\n");
        out.push_str("    cmpb $47, %al\n");
        out.push_str("    je .L_x64_ld_win_star\n");
        out.push_str("    cmpb $92, %al\n");
        out.push_str("    je .L_x64_ld_win_star\n");
        out.push_str("    movb $47, (%rdi)\n");
        out.push_str("    inc %rdi\n");
        out.push_str(".L_x64_ld_win_star:\n");
        out.push_str("    movb $42, (%rdi)\n");
        out.push_str("    movb $0, 1(%rdi)\n");
        out.push_str("    lea 352(%rsp), %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    call FindFirstFileA\n");
        out.push_str("    cmp $-1, %rax\n");
        out.push_str("    je .L_x64_ld_win_ret\n");
        out.push_str("    mov %rax, %r13\n");
        out.push_str(".L_x64_ld_win_loop:\n");
        out.push_str("    lea 76(%rsp), %r14\n");
        out.push_str("    cmpb $46, (%r14)\n");
        out.push_str("    jne .L_x64_ld_win_push\n");
        out.push_str("    cmpb $0, 1(%r14)\n");
        out.push_str("    je .L_x64_ld_win_next\n");
        out.push_str("    cmpb $46, 1(%r14)\n");
        out.push_str("    jne .L_x64_ld_win_push\n");
        out.push_str("    cmpb $0, 2(%r14)\n");
        out.push_str("    je .L_x64_ld_win_next\n");
        out.push_str(".L_x64_ld_win_push:\n");
        out.push_str("    mov %r14, %rcx\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    call alya_array_push\n");
        out.push_str(".L_x64_ld_win_next:\n");
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    call FindNextFileA\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jnz .L_x64_ld_win_loop\n");
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call FindClose\n");
        out.push_str(".L_x64_ld_win_ret:\n");
        out.push_str("    mov %r12, %rax\n");
        out.push_str("    add $888, %rsp\n");
    } else {
        let is_mac = matches!(os, OperatingSystem::MacOS);
        let d_off = if is_mac { 21 } else { 19 };
        out.push_str("    sub $24, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str("    call alya_array_new\n");
        out.push_str("    mov %rax, %r12\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_ld_posix_dot\n");
        out.push_str("    cmpb $0, (%rbx)\n");
        out.push_str("    je .L_x64_ld_posix_dot\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    jmp .L_x64_ld_posix_open\n");
        out.push_str(".L_x64_ld_posix_dot:\n");
        out.push_str("    movw $0x002e, (%rsp)\n");
        out.push_str("    mov %rsp, %rdi\n");
        out.push_str(".L_x64_ld_posix_open:\n");
        out.push_str(&format!("    call {}\n", opendir_fn));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_ld_posix_ret\n");
        out.push_str("    mov %rax, %r13\n");
        out.push_str(".L_x64_ld_posix_loop:\n");
        out.push_str("    mov %r13, %rdi\n");
        out.push_str(&format!("    call {}\n", readdir_fn));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_ld_posix_close\n");
        out.push_str(&format!("    lea {}(%rax), %r14\n", d_off));
        out.push_str("    cmpb $46, (%r14)\n");
        out.push_str("    jne .L_x64_ld_posix_push\n");
        out.push_str("    cmpb $0, 1(%r14)\n");
        out.push_str("    je .L_x64_ld_posix_loop\n");
        out.push_str("    cmpb $46, 1(%r14)\n");
        out.push_str("    jne .L_x64_ld_posix_push\n");
        out.push_str("    cmpb $0, 2(%r14)\n");
        out.push_str("    je .L_x64_ld_posix_loop\n");
        out.push_str(".L_x64_ld_posix_push:\n");
        out.push_str("    mov %r14, %rdi\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    mov %rax, %rsi\n");
        out.push_str("    call alya_array_push\n");
        out.push_str("    jmp .L_x64_ld_posix_loop\n");
        out.push_str(".L_x64_ld_posix_close:\n");
        out.push_str("    mov %r13, %rdi\n");
        out.push_str(&format!("    call {}\n", closedir_fn));
        out.push_str(".L_x64_ld_posix_ret:\n");
        out.push_str("    mov %r12, %rax\n");
        out.push_str("    add $24, %rsp\n");
    }
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

}
