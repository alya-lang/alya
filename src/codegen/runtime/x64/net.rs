use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let is_mac = matches!(os, OperatingSystem::MacOS);
    let p = if is_mac { "_" } else { "" };
    let close_fn = if is_win { "closesocket" } else if is_mac { "_close" } else { "close" };
    let _ = (is_win, is_mac, p, close_fn);

    // fn_net_socket: creates a TCP socket (AF_INET = 2, SOCK_STREAM = 1, 0)
    out.push_str(".global fn_net_socket\n");
    out.push_str("fn_net_socket:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    mov $2, %ecx\n");
        out.push_str("    mov $1, %edx\n");
        out.push_str("    xor %r8d, %r8d\n");
        out.push_str("    call socket\n");
    } else {
        out.push_str("    mov $2, %edi\n");
        out.push_str("    mov $1, %esi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str(&format!("    call {}socket\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jge .L_x64_socket_ok\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_socket_ok:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_connect: connect(host, port) -> socket or -1
    // Win: rcx=host, rdx=port
    // SysV: rdi=host, rsi=port
    out.push_str(".global fn_net_connect\n");
    out.push_str("fn_net_connect:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n"); // 32 bytes shadow/call + 16 sockaddr_in + 8 pad
    if is_win {
        out.push_str("    mov %rcx, %r12\n"); // host
        out.push_str("    mov %rdx, %r13\n"); // port
    } else {
        out.push_str("    mov %rdi, %r12\n"); // host
        out.push_str("    mov %rsi, %r13\n"); // port
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_conn_fail\n");

    // Zero out sockaddr_in at 32(%rsp)
    out.push_str("    movq $0, 32(%rsp)\n");
    out.push_str("    movq $0, 40(%rsp)\n");
    if is_mac {
        out.push_str("    movb $16, 32(%rsp)\n"); // sin_len = 16
        out.push_str("    movb $2, 33(%rsp)\n");  // sin_family = AF_INET (2)
    } else {
        out.push_str("    movw $2, 32(%rsp)\n");  // sin_family = AF_INET (2)
    }
    // htons(port)
    out.push_str("    mov %r13w, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, 34(%rsp)\n");

    // Resolve IP: try inet_addr(host)
    if is_win {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call inet_addr\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}inet_addr\n", p));
    }
    out.push_str("    cmp $0xffffffff, %eax\n");
    out.push_str("    jne .L_x64_conn_have_ip\n");

    // Fallback: gethostbyname(host)
    if is_win {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call gethostbyname\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}gethostbyname\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_conn_fail\n");
    out.push_str("    mov 24(%rax), %rax\n"); // h_addr_list
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_conn_fail\n");
    out.push_str("    mov (%rax), %rax\n");   // h_addr_list[0]
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_conn_fail\n");
    out.push_str("    movl (%rax), %eax\n");  // in_addr.s_addr
    out.push_str(".L_x64_conn_have_ip:\n");
    out.push_str("    movl %eax, 36(%rsp)\n"); // sin_addr.s_addr

    // socket(AF_INET, SOCK_STREAM, 0)
    if is_win {
        out.push_str("    mov $2, %ecx\n");
        out.push_str("    mov $1, %edx\n");
        out.push_str("    xor %r8d, %r8d\n");
        out.push_str("    call socket\n");
    } else {
        out.push_str("    mov $2, %edi\n");
        out.push_str("    mov $1, %esi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str(&format!("    call {}socket\n", p));
    }
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jl .L_x64_conn_fail\n");
    out.push_str("    mov %rax, %rbx\n"); // rbx = socket

    // connect(sock, &sockaddr_in, 16)
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    mov $16, %r8d\n");
        out.push_str("    call connect\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    lea 32(%rsp), %rsi\n");
        out.push_str("    mov $16, %edx\n");
        out.push_str(&format!("    call {}connect\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x64_conn_close_fail\n");
    out.push_str("    mov %rbx, %rax\n"); // return socket
    out.push_str("    jmp .L_x64_conn_ret\n");
    out.push_str(".L_x64_conn_close_fail:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call closesocket\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}\n", close_fn));
    }
    out.push_str(".L_x64_conn_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_conn_ret:\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_listen: net_listen(port, backlog) -> socket or -1
    out.push_str(".global fn_net_listen\n");
    out.push_str("fn_net_listen:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %r12\n"); // port
        out.push_str("    mov %rdx, %r13\n"); // backlog
    } else {
        out.push_str("    mov %rdi, %r12\n"); // port
        out.push_str("    mov %rsi, %r13\n"); // backlog
    }
    out.push_str("    cmp $0, %r13\n");
    out.push_str("    jg .L_x64_listen_bl_ok\n");
    out.push_str("    mov $10, %r13\n");
    out.push_str(".L_x64_listen_bl_ok:\n");

    // socket(AF_INET, SOCK_STREAM, 0)
    if is_win {
        out.push_str("    mov $2, %ecx\n");
        out.push_str("    mov $1, %edx\n");
        out.push_str("    xor %r8d, %r8d\n");
        out.push_str("    call socket\n");
    } else {
        out.push_str("    mov $2, %edi\n");
        out.push_str("    mov $1, %esi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str(&format!("    call {}socket\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jl .L_x64_listen_fail\n");
    out.push_str("    mov %rax, %rbx\n"); // rbx = socket

    // setsockopt SO_REUSEADDR
    out.push_str("    movl $1, 48(%rsp)\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov $0xffff, %edx\n");
        out.push_str("    mov $4, %r8d\n");
        out.push_str("    lea 48(%rsp), %r9\n");
        out.push_str("    movq $4, 32(%rsp)\n");
        out.push_str("    call setsockopt\n");
    } else if is_mac {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov $0xffff, %esi\n");
        out.push_str("    mov $4, %edx\n");
        out.push_str("    lea 48(%rsp), %rcx\n");
        out.push_str("    mov $4, %r8d\n");
        out.push_str(&format!("    call {}setsockopt\n", p));
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov $1, %esi\n");
        out.push_str("    mov $2, %edx\n");
        out.push_str("    lea 48(%rsp), %rcx\n");
        out.push_str("    mov $4, %r8d\n");
        out.push_str("    call setsockopt\n");
    }

    // sockaddr_in at 32(%rsp)
    out.push_str("    movq $0, 32(%rsp)\n");
    out.push_str("    movq $0, 40(%rsp)\n");
    if is_mac {
        out.push_str("    movb $16, 32(%rsp)\n");
        out.push_str("    movb $2, 33(%rsp)\n");
    } else {
        out.push_str("    movw $2, 32(%rsp)\n");
    }
    out.push_str("    mov %r12w, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, 34(%rsp)\n");
    out.push_str("    movl $0, 36(%rsp)\n"); // INADDR_ANY = 0

    // bind(sock, &sin, 16)
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    mov $16, %r8d\n");
        out.push_str("    call bind\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    lea 32(%rsp), %rsi\n");
        out.push_str("    mov $16, %edx\n");
        out.push_str(&format!("    call {}bind\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x64_listen_close_fail\n");

    // listen(sock, backlog)
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    call listen\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r13, %rsi\n");
        out.push_str(&format!("    call {}listen\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x64_listen_close_fail\n");
    out.push_str("    mov %rbx, %rax\n");
    out.push_str("    jmp .L_x64_listen_ret\n");
    out.push_str(".L_x64_listen_close_fail:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call closesocket\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}\n", close_fn));
    }
    out.push_str(".L_x64_listen_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_listen_ret:\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_accept: net_accept(server_sock) -> client_sock or -1
    out.push_str(".global fn_net_accept\n");
    out.push_str("fn_net_accept:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    xor %r8, %r8\n");
        out.push_str("    call accept\n");
    } else {
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str(&format!("    call {}accept\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jge .L_x64_accept_ok\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_accept_ok:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_send: net_send(sock, data_str) -> bytes_sent or -1
    out.push_str(".global fn_net_send\n");
    out.push_str("fn_net_send:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n"); // sock
        out.push_str("    mov %rdx, %r12\n"); // str
    } else {
        out.push_str("    mov %rdi, %rbx\n"); // sock
        out.push_str("    mov %rsi, %r12\n"); // str
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_send_zero\n");
    // strlen
    out.push_str("    mov %r12, %r13\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_send_len:\n");
    out.push_str("    cmpb $0, (%r13)\n");
    out.push_str("    je .L_x64_send_do\n");
    out.push_str("    inc %r13\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_send_len\n");
    out.push_str(".L_x64_send_do:\n");
    out.push_str("    test %r14, %r14\n");
    out.push_str("    jz .L_x64_send_zero\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r12, %rdx\n");
        out.push_str("    mov %r14, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    call send\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r12, %rsi\n");
        out.push_str("    mov %r14, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str(&format!("    call {}send\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    jmp .L_x64_send_ret\n");
    out.push_str(".L_x64_send_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_send_ret:\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_recv: net_recv(sock, max_bytes) -> string
    out.push_str(".global fn_net_recv\n");
    out.push_str("fn_net_recv:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n"); // sock
        out.push_str("    mov %rdx, %r13\n"); // max_bytes
    } else {
        out.push_str("    mov %rdi, %rbx\n"); // sock
        out.push_str("    mov %rsi, %r13\n"); // max_bytes
    }
    out.push_str("    cmp $0, %r13\n");
    out.push_str("    jg .L_x64_recv_chk\n");
    out.push_str("    mov $4096, %r13\n");
    out.push_str(".L_x64_recv_chk:\n");
    out.push_str("    cmp $524288, %r13\n");
    out.push_str("    jle .L_x64_recv_alloc\n");
    out.push_str("    mov $524288, %r13\n");
    out.push_str(".L_x64_recv_alloc:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %r14\n");
    out.push_str("    mov $1000000, %r11\n");
    out.push_str("    sub %r13, %r11\n");
    out.push_str("    cmp %r11, %r14\n");
    out.push_str("    jb .L_x64_recv_buf_ok\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_recv_buf_ok:\n");
    out.push_str("    lea (%r8, %r14), %r12\n"); // destination buffer

    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r12, %rdx\n");
        out.push_str("    mov %r13, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    call recv\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r12, %rsi\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str(&format!("    call {}recv\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x64_recv_empty\n");
    out.push_str("    cltq\n");
    out.push_str("    movb $0, (%r12, %rax)\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    lea 1(%rax, %r12), %r10\n");
    out.push_str("    sub %r8, %r10\n");
    out.push_str("    add $7, %r10\n");
    out.push_str("    and $-8, %r10\n");
    out.push_str("    mov %r10, (%r9)\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    jmp .L_x64_recv_done\n");
    out.push_str(".L_x64_recv_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_recv_done:\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_close: net_close(sock) -> 0
    out.push_str(".global fn_net_close\n");
    out.push_str("fn_net_close:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    call closesocket\n");
    } else {
        out.push_str(&format!("    call {}\n", close_fn));
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_set_timeout: net_set_timeout(sock, ms) -> 0 or -1
    out.push_str(".global fn_net_set_timeout\n");
    out.push_str("fn_net_set_timeout:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    movl %edx, 48(%rsp)\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov $0xffff, %edx\n");
        out.push_str("    mov $0x1006, %r8d\n");
        out.push_str("    lea 48(%rsp), %r9\n");
        out.push_str("    movq $4, 32(%rsp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_timeout_fail\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov $0xffff, %edx\n");
        out.push_str("    mov $0x1005, %r8d\n");
        out.push_str("    lea 48(%rsp), %r9\n");
        out.push_str("    movq $4, 32(%rsp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_timeout_fail\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %rax\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %rcx\n");
        out.push_str("    imul $1000, %rdx, %rdx\n");
        out.push_str("    movq %rax, 32(%rsp)\n");
        out.push_str("    movq %rdx, 40(%rsp)\n");
        let (sol, rcv_opt, snd_opt) = if is_mac {
            (0xffff, 0x1006, 0x1005)
        } else {
            (1, 20, 21)
        };
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    mov ${}, %esi\n", sol));
        out.push_str(&format!("    mov ${}, %edx\n", rcv_opt));
        out.push_str("    lea 32(%rsp), %rcx\n");
        out.push_str("    mov $16, %r8d\n");
        out.push_str(&format!("    call {}setsockopt\n", p));
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_timeout_fail\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    mov ${}, %esi\n", sol));
        out.push_str(&format!("    mov ${}, %edx\n", snd_opt));
        out.push_str("    lea 32(%rsp), %rcx\n");
        out.push_str("    mov $16, %r8d\n");
        out.push_str(&format!("    call {}setsockopt\n", p));
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_timeout_fail\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    jmp .L_x64_timeout_ret\n");
    out.push_str(".L_x64_timeout_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_timeout_ret:\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_ip: net_peer_ip(sock) -> ip_string or ""
    out.push_str(".global fn_net_peer_ip\n");
    out.push_str("fn_net_peer_ip:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    sub $64, %rsp\n");
    out.push_str("    movl $16, 56(%rsp)\n");
    if is_win {
        out.push_str("    lea 40(%rsp), %rdx\n");
        out.push_str("    lea 56(%rsp), %r8\n");
        out.push_str("    call getpeername\n");
    } else {
        out.push_str("    lea 40(%rsp), %rsi\n");
        out.push_str("    lea 56(%rsp), %rdx\n");
        out.push_str(&format!("    call {}getpeername\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jne .L_x64_peer_ip_empty\n");
    if is_win {
        out.push_str("    movl 44(%rsp), %ecx\n");
        out.push_str("    call inet_ntoa\n");
    } else {
        out.push_str("    movl 44(%rsp), %edi\n");
        out.push_str(&format!("    call {}inet_ntoa\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_peer_ip_empty\n");
    out.push_str("    mov %rax, %r10\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_peer_ip_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_peer_ip_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r11\n");
    out.push_str("    mov %r11, %r12\n");
    out.push_str(".L_x64_peer_ip_copy:\n");
    out.push_str("    movb (%r10), %al\n");
    out.push_str("    movb %al, (%r11)\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x64_peer_ip_copy_done\n");
    out.push_str("    inc %r10\n");
    out.push_str("    inc %r11\n");
    out.push_str("    jmp .L_x64_peer_ip_copy\n");
    out.push_str(".L_x64_peer_ip_copy_done:\n");
    out.push_str("    inc %r11\n");
    out.push_str("    sub %r8, %r11\n");
    out.push_str("    add $7, %r11\n");
    out.push_str("    and $-8, %r11\n");
    out.push_str("    mov %r11, (%r9)\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    jmp .L_x64_peer_ip_ret\n");
    out.push_str(".L_x64_peer_ip_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_peer_ip_ret:\n");
    out.push_str("    add $64, %rsp\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_port: net_peer_port(sock) -> port_int or -1
    out.push_str(".global fn_net_peer_port\n");
    out.push_str("fn_net_peer_port:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    out.push_str("    movl $16, 48(%rsp)\n");
    if is_win {
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    lea 48(%rsp), %r8\n");
        out.push_str("    call getpeername\n");
    } else {
        out.push_str("    lea 32(%rsp), %rsi\n");
        out.push_str("    lea 48(%rsp), %rdx\n");
        out.push_str(&format!("    call {}getpeername\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jne .L_x64_peer_port_fail\n");
    out.push_str("    movzwl 34(%rsp), %eax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    jmp .L_x64_peer_port_ret\n");
    out.push_str(".L_x64_peer_port_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_peer_port_ret:\n");
    out.push_str("    add $64, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_socket: creates a UDP socket (AF_INET = 2, SOCK_DGRAM = 2, 0)
    out.push_str(".global fn_net_udp_socket\n");
    out.push_str("fn_net_udp_socket:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    mov $2, %ecx\n");
        out.push_str("    mov $2, %edx\n");
        out.push_str("    xor %r8d, %r8d\n");
        out.push_str("    call socket\n");
    } else {
        out.push_str("    mov $2, %edi\n");
        out.push_str("    mov $2, %esi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str(&format!("    call {}socket\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    cmp $0, %rax\n");
    out.push_str("    jge .L_x64_udp_socket_ok\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_udp_socket_ok:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_bind: binds UDP socket to port -> 0 or -1
    out.push_str(".global fn_net_udp_bind\n");
    out.push_str("fn_net_udp_bind:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    mov %rdx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r12\n");
    }
    out.push_str("    movq $0, 32(%rsp)\n");
    out.push_str("    movq $0, 40(%rsp)\n");
    if is_mac {
        out.push_str("    movb $16, 32(%rsp)\n");
        out.push_str("    movb $2, 33(%rsp)\n");
    } else {
        out.push_str("    movw $2, 32(%rsp)\n");
    }
    out.push_str("    mov %r12w, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, 34(%rsp)\n");
    out.push_str("    movl $0, 36(%rsp)\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    mov $16, %r8d\n");
        out.push_str("    call bind\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    lea 32(%rsp), %rsi\n");
        out.push_str("    mov $16, %edx\n");
        out.push_str(&format!("    call {}bind\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x64_udp_bind_fail\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    jmp .L_x64_udp_bind_ret\n");
    out.push_str(".L_x64_udp_bind_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_udp_bind_ret:\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_send: net_udp_send(sock, host, port, data_str) -> bytes_sent or -1
    out.push_str(".global fn_net_udp_send\n");
    out.push_str("fn_net_udp_send:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $72, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    mov %rdx, %r12\n");
        out.push_str("    mov %r8, %r13\n");
        out.push_str("    mov %r9, %r14\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    mov %rcx, %r14\n");
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_usend_fail\n");
    out.push_str("    movq $0, 48(%rsp)\n");
    out.push_str("    movq $0, 56(%rsp)\n");
    if is_mac {
        out.push_str("    movb $16, 48(%rsp)\n");
        out.push_str("    movb $2, 49(%rsp)\n");
    } else {
        out.push_str("    movw $2, 48(%rsp)\n");
    }
    out.push_str("    mov %r13w, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, 50(%rsp)\n");
    if is_win {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call inet_addr\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}inet_addr\n", p));
    }
    out.push_str("    cmp $0xffffffff, %eax\n");
    out.push_str("    jne .L_x64_usend_have_ip\n");
    if is_win {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call gethostbyname\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}gethostbyname\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_usend_fail\n");
    out.push_str("    mov 24(%rax), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_usend_fail\n");
    out.push_str("    mov (%rax), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_usend_fail\n");
    out.push_str("    movl (%rax), %eax\n");
    out.push_str(".L_x64_usend_have_ip:\n");
    out.push_str("    movl %eax, 52(%rsp)\n");
    out.push_str("    xor %r15, %r15\n");
    out.push_str("    test %r14, %r14\n");
    out.push_str("    jz .L_x64_usend_call\n");
    out.push_str("    mov %r14, %rax\n");
    out.push_str(".L_x64_usend_len_loop:\n");
    out.push_str("    cmpb $0, (%rax)\n");
    out.push_str("    je .L_x64_usend_call\n");
    out.push_str("    inc %rax\n");
    out.push_str("    inc %r15\n");
    out.push_str("    jmp .L_x64_usend_len_loop\n");
    out.push_str(".L_x64_usend_call:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r14, %rdx\n");
        out.push_str("    mov %r15, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    lea 48(%rsp), %rax\n");
        out.push_str("    movq %rax, 32(%rsp)\n");
        out.push_str("    movq $16, 40(%rsp)\n");
        out.push_str("    call sendto\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r14, %rsi\n");
        out.push_str("    mov %r15, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    lea 48(%rsp), %r8\n");
        out.push_str("    mov $16, %r9d\n");
        out.push_str(&format!("    call {}sendto\n", p));
    }
    out.push_str("    cltq\n");
    out.push_str("    jmp .L_x64_usend_ret\n");
    out.push_str(".L_x64_usend_fail:\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str(".L_x64_usend_ret:\n");
    out.push_str("    add $72, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_recv: net_udp_recv(sock, max_bytes) -> string
    out.push_str(".global fn_net_udp_recv\n");
    out.push_str("fn_net_udp_recv:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    cmp $0, %r13\n");
    out.push_str("    jg .L_x64_urecv_chk\n");
    out.push_str("    mov $4096, %r13\n");
    out.push_str(".L_x64_urecv_chk:\n");
    out.push_str("    cmp $524288, %r13\n");
    out.push_str("    jle .L_x64_urecv_alloc\n");
    out.push_str("    mov $524288, %r13\n");
    out.push_str(".L_x64_urecv_alloc:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %r14\n");
    out.push_str("    mov $1000000, %r11\n");
    out.push_str("    sub %r13, %r11\n");
    out.push_str("    cmp %r11, %r14\n");
    out.push_str("    jb .L_x64_urecv_buf_ok\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_urecv_buf_ok:\n");
    out.push_str("    lea (%r8, %r14), %r12\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r12, %rdx\n");
        out.push_str("    mov %r13, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    movq $0, 32(%rsp)\n");
        out.push_str("    movq $0, 40(%rsp)\n");
        out.push_str("    call recvfrom\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r12, %rsi\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    xor %r8, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str(&format!("    call {}recvfrom\n", p));
    }
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x64_urecv_empty\n");
    out.push_str("    cltq\n");
    out.push_str("    movb $0, (%r12, %rax)\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    lea 1(%rax, %r12), %r10\n");
    out.push_str("    sub %r8, %r10\n");
    out.push_str("    add $7, %r10\n");
    out.push_str("    and $-8, %r10\n");
    out.push_str("    mov %r10, (%r9)\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    jmp .L_x64_urecv_done\n");
    out.push_str(".L_x64_urecv_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_urecv_done:\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_set_nonblocking: net_set_nonblocking(sock, mode) -> 0 or -1
    out.push_str(".global fn_net_set_nonblocking\n");
    out.push_str("fn_net_set_nonblocking:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $48, %rsp\n");
        out.push_str("    movl %edx, 32(%rsp)\n");
        out.push_str("    mov $0x8004667e, %edx\n");
        out.push_str("    lea 32(%rsp), %r8\n");
        out.push_str("    call ioctlsocket\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jge .L_x64_snb_ok\n");
        out.push_str("    mov $-1, %rax\n");
        out.push_str("    jmp .L_x64_snb_ret\n");
        out.push_str(".L_x64_snb_ok:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_snb_ret:\n");
        out.push_str("    add $48, %rsp\n");
    } else {
        let nonblock_flag = if is_mac { 4 } else { 2048 };
        out.push_str("    push %rbx\n");
        out.push_str("    push %r12\n");
        out.push_str("    sub $16, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r12\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov $3, %esi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str(&format!("    call {}fcntl\n", p));
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_snb_err\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jz .L_x64_snb_clear\n");
        out.push_str(&format!("    or ${}, %eax\n", nonblock_flag));
        out.push_str("    jmp .L_x64_snb_set\n");
        out.push_str(".L_x64_snb_clear:\n");
        out.push_str(&format!("    and ${}, %eax\n", !nonblock_flag));
        out.push_str(".L_x64_snb_set:\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov $4, %esi\n");
        out.push_str("    mov %eax, %edx\n");
        out.push_str(&format!("    call {}fcntl\n", p));
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x64_snb_err\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    jmp .L_x64_snb_done\n");
        out.push_str(".L_x64_snb_err:\n");
        out.push_str("    mov $-1, %rax\n");
        out.push_str(".L_x64_snb_done:\n");
        out.push_str("    add $16, %rsp\n");
        out.push_str("    pop %r12\n");
        out.push_str("    pop %rbx\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_net_poll: net_poll(sock, timeout_ms) -> 1 (ready), 0 (timeout), -1 (error)
    out.push_str(".global fn_net_poll\n");
    out.push_str("fn_net_poll:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $80, %rsp\n");
        out.push_str("    mov %rcx, %r8\n");
        out.push_str("    mov %rdx, %r10\n");
        out.push_str("    movl $1, 48(%rsp)\n");
        out.push_str("    movl $0, 52(%rsp)\n");
        out.push_str("    movq %r8, 56(%rsp)\n");

        out.push_str("    cmp $0, %r10\n");
        out.push_str("    jl .L_x64_poll_win_inf\n");
        out.push_str("    mov %r10, %rax\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    movl %eax, 40(%rsp)\n");
        out.push_str("    imul $1000, %edx, %edx\n");
        out.push_str("    movl %edx, 44(%rsp)\n");
        out.push_str("    lea 40(%rsp), %rax\n");
        out.push_str("    movq %rax, 32(%rsp)\n");
        out.push_str("    jmp .L_x64_poll_win_call\n");
        out.push_str(".L_x64_poll_win_inf:\n");
        out.push_str("    movq $0, 32(%rsp)\n");
        out.push_str(".L_x64_poll_win_call:\n");
        out.push_str("    xor %ecx, %ecx\n");
        out.push_str("    lea 48(%rsp), %rdx\n");
        out.push_str("    xor %r8, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    call select\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jg .L_x64_poll_win_ready\n");
        out.push_str("    jl .L_x64_poll_win_err\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    jmp .L_x64_poll_win_done\n");
        out.push_str(".L_x64_poll_win_ready:\n");
        out.push_str("    mov $1, %rax\n");
        out.push_str("    jmp .L_x64_poll_win_done\n");
        out.push_str(".L_x64_poll_win_err:\n");
        out.push_str("    mov $-1, %rax\n");
        out.push_str(".L_x64_poll_win_done:\n");
        out.push_str("    add $80, %rsp\n");
    } else {
        out.push_str("    push %rbx\n");
        out.push_str("    push %r12\n");
        out.push_str("    sub $160, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r12\n");

        out.push_str("    lea 16(%rsp), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    mov $16, %ecx\n");
        out.push_str("    rep stosq\n");

        out.push_str("    mov %rbx, %rax\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    shr $6, %rax\n");
        out.push_str("    and $63, %rcx\n");
        out.push_str("    bts %rcx, 16(%rsp, %rax, 8)\n");

        out.push_str("    cmp $0, %r12\n");
        out.push_str("    jl .L_x64_poll_posix_inf\n");
        out.push_str("    mov %r12, %rax\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    movq %rax, 0(%rsp)\n");
        out.push_str("    imul $1000, %edx, %edx\n");
        out.push_str("    movslq %edx, %rdx\n");
        out.push_str("    movq %rdx, 8(%rsp)\n");
        out.push_str("    lea 0(%rsp), %r8\n");
        out.push_str("    jmp .L_x64_poll_posix_call\n");
        out.push_str(".L_x64_poll_posix_inf:\n");
        out.push_str("    xor %r8, %r8\n");
        out.push_str(".L_x64_poll_posix_call:\n");
        out.push_str("    lea 1(%rbx), %rdi\n");
        out.push_str("    lea 16(%rsp), %rsi\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    xor %ecx, %ecx\n");
        out.push_str(&format!("    call {}select\n", p));
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jg .L_x64_poll_posix_ready\n");
        out.push_str("    jl .L_x64_poll_posix_err\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    jmp .L_x64_poll_posix_done\n");
        out.push_str(".L_x64_poll_posix_ready:\n");
        out.push_str("    mov $1, %rax\n");
        out.push_str("    jmp .L_x64_poll_posix_done\n");
        out.push_str(".L_x64_poll_posix_err:\n");
        out.push_str("    mov $-1, %rax\n");
        out.push_str(".L_x64_poll_posix_done:\n");
        out.push_str("    add $160, %rsp\n");
        out.push_str("    pop %r12\n");
        out.push_str("    pop %rbx\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
