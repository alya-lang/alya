use crate::codegen::target::OperatingSystem;
use super::{emit_str_buf_load, emit_str_buf_store};

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let close_fn = if is_win { "closesocket" } else { "close" };
    let _ = (is_win, close_fn);

    // fn_net_socket: TCP socket (2, 1, 0)
    out.push_str(".global fn_net_socket\n");
    out.push_str("fn_net_socket:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push $0\n");
    out.push_str("    push $1\n");
    out.push_str("    push $2\n");
    out.push_str("    call socket\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jge .L_x86_socket_ok\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_socket_ok:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_connect: connect(host, port) -> socket or -1
    out.push_str(".global fn_net_connect\n");
    out.push_str("fn_net_connect:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $32, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");  // host
    out.push_str("    mov 12(%ebp), %ebx\n"); // port
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_conn_fail\n");

    // sockaddr_in at -44(%ebp)
    out.push_str("    movl $0, -44(%ebp)\n");
    out.push_str("    movl $0, -40(%ebp)\n");
    out.push_str("    movl $0, -36(%ebp)\n");
    out.push_str("    movl $0, -32(%ebp)\n");
    out.push_str("    movw $2, -44(%ebp)\n"); // AF_INET
    out.push_str("    mov %bx, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, -42(%ebp)\n"); // sin_port

    // inet_addr(host)
    out.push_str("    push %esi\n");
    out.push_str("    call inet_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmp $0xffffffff, %eax\n");
    out.push_str("    jne .L_x86_conn_have_ip\n");

    // gethostbyname(host)
    out.push_str("    push %esi\n");
    out.push_str("    call gethostbyname\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_conn_fail\n");
    out.push_str("    mov 16(%eax), %eax\n"); // h_addr_list (offset 16 on 32-bit)
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_conn_fail\n");
    out.push_str("    mov (%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_conn_fail\n");
    out.push_str("    movl (%eax), %eax\n");
    out.push_str(".L_x86_conn_have_ip:\n");
    out.push_str("    movl %eax, -40(%ebp)\n");

    // socket(2, 1, 0)
    out.push_str("    push $0\n");
    out.push_str("    push $1\n");
    out.push_str("    push $2\n");
    out.push_str("    call socket\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_conn_fail\n");
    out.push_str("    mov %eax, %ebx\n");

    // connect(sock, &sin, 16)
    out.push_str("    push $16\n");
    out.push_str("    lea -44(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call connect\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_conn_close\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    jmp .L_x86_conn_ret\n");
    out.push_str(".L_x86_conn_close:\n");
    out.push_str("    push %ebx\n");
    out.push_str(&format!("    call {}\n", close_fn));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_conn_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_conn_ret:\n");
    out.push_str("    add $32, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_listen: net_listen(port, backlog) -> socket or -1
    out.push_str(".global fn_net_listen\n");
    out.push_str("fn_net_listen:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $32, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");  // port
    out.push_str("    mov 12(%ebp), %edi\n"); // backlog
    out.push_str("    cmp $0, %edi\n");
    out.push_str("    jg .L_x86_listen_bl_ok\n");
    out.push_str("    mov $10, %edi\n");
    out.push_str(".L_x86_listen_bl_ok:\n");

    // socket(2, 1, 0)
    out.push_str("    push $0\n");
    out.push_str("    push $1\n");
    out.push_str("    push $2\n");
    out.push_str("    call socket\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_listen_fail\n");
    out.push_str("    mov %eax, %ebx\n");

    // setsockopt(SO_REUSEADDR), mirroring x64: lets tests rebind the
    // same port across rapid runs (TIME_WAIT otherwise fails bind).
    out.push_str("    movl $1, -16(%ebp)\n");
    out.push_str("    push $4\n");
    out.push_str("    lea -16(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $2\n");
    out.push_str("    push $1\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call setsockopt\n");
    out.push_str("    add $20, %esp\n");

    // sockaddr_in
    out.push_str("    movl $0, -44(%ebp)\n");
    out.push_str("    movl $0, -40(%ebp)\n");
    out.push_str("    movl $0, -36(%ebp)\n");
    out.push_str("    movl $0, -32(%ebp)\n");
    out.push_str("    movw $2, -44(%ebp)\n");
    out.push_str("    mov %si, %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, -42(%ebp)\n");
    out.push_str("    movl $0, -40(%ebp)\n");

    // bind(sock, &sin, 16)
    out.push_str("    push $16\n");
    out.push_str("    lea -44(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call bind\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_listen_close\n");

    // listen(sock, backlog)
    out.push_str("    push %edi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call listen\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_listen_close\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    jmp .L_x86_listen_ret\n");
    out.push_str(".L_x86_listen_close:\n");
    out.push_str("    push %ebx\n");
    out.push_str(&format!("    call {}\n", close_fn));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_listen_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_listen_ret:\n");
    out.push_str("    add $32, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_accept: net_accept(server_sock) -> client_sock or -1
    out.push_str(".global fn_net_accept\n");
    out.push_str("fn_net_accept:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push $0\n");
    out.push_str("    push $0\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call accept\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jge .L_x86_accept_ok\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_accept_ok:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_send: net_send(sock, data_str) -> bytes_sent or -1
    out.push_str(".global fn_net_send\n");
    out.push_str("fn_net_send:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // str
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_send_zero\n");
    // strlen
    out.push_str("    mov %esi, %edi\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_send_len:\n");
    out.push_str("    cmpb $0, (%edi)\n");
    out.push_str("    je .L_x86_send_do\n");
    out.push_str("    inc %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_send_len\n");
    out.push_str(".L_x86_send_do:\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_send_zero\n");
    out.push_str("    push $0\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call send\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    jmp .L_x86_send_ret\n");
    out.push_str(".L_x86_send_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_send_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_recv: net_recv(sock, max_bytes) -> string
    out.push_str(".global fn_net_recv\n");
    out.push_str("fn_net_recv:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // max_bytes
    out.push_str("    cmp $0, %esi\n");
    out.push_str("    jg .L_x86_recv_chk\n");
    out.push_str("    mov $4096, %esi\n");
    out.push_str(".L_x86_recv_chk:\n");
    out.push_str("    cmp $524288, %esi\n");
    out.push_str("    jle .L_x86_recv_alloc\n");
    out.push_str("    mov $524288, %esi\n");
    out.push_str(".L_x86_recv_alloc:\n");
    emit_str_buf_load(out, "%edx", "%edi", os);
    out.push_str("    mov $1000000, %ecx\n");
    out.push_str("    sub %esi, %ecx\n");
    out.push_str("    cmp %ecx, %edi\n");
    out.push_str("    jl .L_x86_recv_buf_ok\n");
    out.push_str("    xor %edi, %edi\n");
    out.push_str(".L_x86_recv_buf_ok:\n");
    out.push_str("    add %edi, %edx\n");
    out.push_str("    mov %edx, -4(%ebp)\n");
    out.push_str("    push $0\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call recv\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x86_recv_empty\n");
    out.push_str("    mov -4(%ebp), %edx\n");
    out.push_str("    movb $0, (%edx, %eax)\n");
    out.push_str("    lea 1(%eax, %edi), %ecx\n");
    out.push_str("    add $3, %ecx\n");
    out.push_str("    and $-4, %ecx\n");
    emit_str_buf_store(out, "%ecx", "%esi", os);
    out.push_str("    mov %edx, %eax\n");
    out.push_str("    jmp .L_x86_recv_done\n");
    out.push_str(".L_x86_recv_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_recv_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_send_bytes: net_send_bytes(sock, array) -> bytes_sent or -1.
    // Sends exactly len(array) bytes (low byte of each element slot);
    // embedded zeros are preserved (no strlen). Null/non-array/empty
    // input sends nothing and returns 0; malloc failure returns -1.
    out.push_str(".global fn_net_send_bytes\n");
    out.push_str("fn_net_send_bytes:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // array
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_sendb_zero\n");
    out.push_str("    movl -8(%esi), %eax\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    jne .L_x86_sendb_err\n");
    out.push_str("    movl (%esi), %ecx\n"); // len
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_sendb_zero\n");
    out.push_str("    movl 8(%esi), %edi\n"); // data
    // Locals live below the saved registers (which occupy -4..-12):
    // using -4/-8 would clobber the caller's %ebx/%esi.
    out.push_str("    mov %ecx, -16(%ebp)\n");
    out.push_str("    push %ecx\n");
    out.push_str("    call malloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_sendb_err\n");
    out.push_str("    mov %eax, -20(%ebp)\n"); // tmp
    out.push_str("    mov -20(%ebp), %edx\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_sendb_pack:\n");
    out.push_str("    cmp -16(%ebp), %ecx\n");
    out.push_str("    jge .L_x86_sendb_do\n");
    out.push_str("    movzbl (%edi, %ecx, 8), %eax\n");
    out.push_str("    mov %al, (%edx, %ecx)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_sendb_pack\n");
    out.push_str(".L_x86_sendb_do:\n");
    out.push_str("    push $0\n");
    out.push_str("    push -16(%ebp)\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call send\n");
    out.push_str("    add $16, %esp\n");
    // `sent` must survive `free` below: %ecx is caller-saved and free
    // clobbers it, so spill to a local slot (-24 is free in sub $16).
    out.push_str("    mov %eax, -24(%ebp)\n");
    out.push_str("    mov -20(%ebp), %edx\n");
    out.push_str("    push %edx\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -24(%ebp), %eax\n"); // sent
    out.push_str("    jmp .L_x86_sendb_ret\n");
    out.push_str(".L_x86_sendb_err:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str("    jmp .L_x86_sendb_ret\n");
    out.push_str(".L_x86_sendb_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_sendb_ret:\n");
    // Restore the stack past the $16 locals, then the saved registers
    // (popping after `mov %ebp,%esp` would read the caller frame).
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_recv_bytes: net_recv_bytes(sock, max_bytes) -> int array.
    // Receives up to max_bytes into a scratch buffer, then expands each
    // byte into an array slot (0-255 as ints). Embedded zeros preserved.
    // Close/EOF, errors, and malloc failure all yield an empty array.
    out.push_str(".global fn_net_recv_bytes\n");
    out.push_str("fn_net_recv_bytes:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // max_bytes
    out.push_str("    cmp $0, %esi\n");
    out.push_str("    jg .L_x86_recvb_chk\n");
    out.push_str("    mov $4096, %esi\n");
    out.push_str(".L_x86_recvb_chk:\n");
    out.push_str("    cmp $524288, %esi\n");
    out.push_str("    jle .L_x86_recvb_alloc\n");
    out.push_str("    mov $524288, %esi\n");
    out.push_str(".L_x86_recvb_alloc:\n");
    out.push_str("    push %esi\n");
    out.push_str("    call malloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_recvb_empty_nomem\n");
    out.push_str("    mov %eax, %edi\n"); // tmp
    out.push_str("    push $0\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call recv\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x86_recvb_empty\n");
    out.push_str("    mov %eax, %esi\n"); // n
    out.push_str("    push %esi\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    // Spill the handle: %edx is caller-saved and `free` below clobbers
    // it (same bug class as fn_net_send_bytes' `sent`). -4(%ebp) is
    // free (sub-area, saves live below it).
    out.push_str("    mov %eax, -4(%ebp)\n"); // handle
    out.push_str("    mov 8(%eax), %ebx\n"); // data
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_recvb_fill:\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_recvb_done_fill\n");
    out.push_str("    movzbl (%edi, %ecx), %eax\n");
    // 8-byte element slots (slots are zeroed by calloc; kinds stay
    // unknown so reads fall back to integer classification).
    out.push_str("    mov %eax, (%ebx, %ecx, 8)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_recvb_fill\n");
    out.push_str(".L_x86_recvb_done_fill:\n");
    out.push_str("    push %edi\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -4(%ebp), %eax\n");
    out.push_str("    jmp .L_x86_recvb_done\n");
    out.push_str(".L_x86_recvb_empty:\n");
    out.push_str("    push %edi\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_recvb_empty_nomem:\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_recvb_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_send_bytes: net_udp_send_bytes(sock, host, port, array).
    // Sends exactly len(array) bytes via sendto (low byte of each slot);
    // embedded zeros are preserved (no strlen). Null host fails (-1);
    // null/non-array/empty input sends nothing (0); malloc failure (-1).
    out.push_str(".global fn_net_udp_send_bytes\n");
    out.push_str("fn_net_udp_send_bytes:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $28, %esp\n");
    out.push_str("    mov 12(%ebp), %esi\n"); // host
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_usendb_fail\n");
    out.push_str("    mov 20(%ebp), %eax\n"); // array
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usendb_zero\n");
    out.push_str("    movl -8(%eax), %ecx\n");
    out.push_str("    cmpl $0x5A110001, %ecx\n");
    out.push_str("    jne .L_x86_usendb_err\n");
    out.push_str("    movl (%eax), %ecx\n"); // len
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_usendb_zero\n");
    out.push_str("    movl 8(%eax), %edi\n"); // data
    out.push_str("    mov %ecx, -32(%ebp)\n");
    out.push_str("    movl $0, -28(%ebp)\n");
    out.push_str("    movl $0, -24(%ebp)\n");
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str("    movw $2, -28(%ebp)\n");
    out.push_str("    mov 16(%ebp), %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, -26(%ebp)\n");
    out.push_str("    push %esi\n");
    out.push_str("    call inet_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmp $0xffffffff, %eax\n");
    out.push_str("    jne .L_x86_usendb_have_ip\n");
    out.push_str("    push %esi\n");
    out.push_str("    call gethostbyname\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usendb_fail\n");
    out.push_str("    mov 16(%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usendb_fail\n");
    out.push_str("    mov (%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usendb_fail\n");
    out.push_str("    movl (%eax), %eax\n");
    out.push_str(".L_x86_usendb_have_ip:\n");
    out.push_str("    movl %eax, -24(%ebp)\n");
    out.push_str("    push -32(%ebp)\n");
    out.push_str("    call malloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usendb_err\n");
    out.push_str("    mov %eax, -36(%ebp)\n"); // tmp
    out.push_str("    mov -36(%ebp), %edx\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_usendb_pack:\n");
    out.push_str("    cmp -32(%ebp), %ecx\n");
    out.push_str("    jge .L_x86_usendb_call\n");
    out.push_str("    movzbl (%edi, %ecx, 8), %eax\n");
    out.push_str("    mov %al, (%edx, %ecx)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_usendb_pack\n");
    out.push_str(".L_x86_usendb_call:\n");
    out.push_str("    push $16\n");
    out.push_str("    lea -28(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push -32(%ebp)\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call sendto\n");
    out.push_str("    add $24, %esp\n");
    // Spill across `free` like fn_net_send_bytes (-40 is free in sub $28).
    out.push_str("    mov %eax, -40(%ebp)\n"); // sent
    out.push_str("    mov -36(%ebp), %edx\n");
    out.push_str("    push %edx\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -40(%ebp), %eax\n");
    out.push_str("    jmp .L_x86_usendb_ret\n");
    out.push_str(".L_x86_usendb_err:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str("    jmp .L_x86_usendb_ret\n");
    out.push_str(".L_x86_usendb_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str("    jmp .L_x86_usendb_ret\n");
    out.push_str(".L_x86_usendb_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_usendb_ret:\n");
    // Same frame discipline as fn_net_send_bytes above (sub $28).
    out.push_str("    add $28, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_recv_bytes: net_udp_recv_bytes(sock, max_bytes).
    // Receives one datagram (up to max_bytes) and expands each byte into
    // an array slot (0-255 as ints). Embedded zeros preserved. Errors
    // and malloc failure yield an empty array.
    out.push_str(".global fn_net_udp_recv_bytes\n");
    out.push_str("fn_net_udp_recv_bytes:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // max_bytes
    out.push_str("    cmp $0, %esi\n");
    out.push_str("    jg .L_x86_urecvb_chk\n");
    out.push_str("    mov $4096, %esi\n");
    out.push_str(".L_x86_urecvb_chk:\n");
    out.push_str("    cmp $524288, %esi\n");
    out.push_str("    jle .L_x86_urecvb_alloc\n");
    out.push_str("    mov $524288, %esi\n");
    out.push_str(".L_x86_urecvb_alloc:\n");
    out.push_str("    push %esi\n");
    out.push_str("    call malloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_urecvb_empty_nomem\n");
    out.push_str("    mov %eax, %edi\n"); // tmp
    out.push_str("    push $0\n"); // addrlen = NULL
    out.push_str("    push $0\n"); // src_addr = NULL
    out.push_str("    push $0\n"); // flags = 0
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call recvfrom\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x86_urecvb_empty\n");
    out.push_str("    mov %eax, %esi\n"); // n
    out.push_str("    push %esi\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    // Spill the handle across `free` like fn_net_recv_bytes (-4 free).
    out.push_str("    mov %eax, -4(%ebp)\n"); // handle
    out.push_str("    mov 8(%eax), %ebx\n"); // data
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_urecvb_fill:\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_urecvb_done_fill\n");
    out.push_str("    movzbl (%edi, %ecx), %eax\n");
    out.push_str("    mov %eax, (%ebx, %ecx, 8)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_urecvb_fill\n");
    out.push_str(".L_x86_urecvb_done_fill:\n");
    out.push_str("    push %edi\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -4(%ebp), %eax\n");
    out.push_str("    jmp .L_x86_urecvb_done\n");
    out.push_str(".L_x86_urecvb_empty:\n");
    out.push_str("    push %edi\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_urecvb_empty_nomem:\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_urecvb_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_close: net_close(sock) -> 0
    out.push_str(".global fn_net_close\n");
    out.push_str("fn_net_close:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str(&format!("    call {}\n", close_fn));
    out.push_str("    add $4, %esp\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_set_timeout: net_set_timeout(sock, ms) -> 0 or -1
    out.push_str(".global fn_net_set_timeout\n");
    out.push_str("fn_net_set_timeout:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    if is_win {
        // SO_RCVTIMEO
        out.push_str("    push $4\n");
        out.push_str("    lea 12(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $0x1006\n");
        out.push_str("    push $0xffff\n");
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_timeout_fail\n");
        // SO_SNDTIMEO
        out.push_str("    push $4\n");
        out.push_str("    lea 12(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $0x1005\n");
        out.push_str("    push $0xffff\n");
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_timeout_fail\n");
    } else {
        out.push_str("    mov 12(%ebp), %eax\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    imul $1000, %edx, %edx\n");
        out.push_str("    mov %eax, -8(%ebp)\n");
        out.push_str("    mov %edx, -4(%ebp)\n");
        // SO_RCVTIMEO
        out.push_str("    push $8\n");
        out.push_str("    lea -8(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $20\n");
        out.push_str("    push $1\n");
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_timeout_fail\n");
        // SO_SNDTIMEO
        out.push_str("    push $8\n");
        out.push_str("    lea -8(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $21\n");
        out.push_str("    push $1\n");
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call setsockopt\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_timeout_fail\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    jmp .L_x86_timeout_ret\n");
    out.push_str(".L_x86_timeout_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_timeout_ret:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_ip: net_peer_ip(sock) -> ip_string or ""
    out.push_str(".global fn_net_peer_ip\n");
    out.push_str("fn_net_peer_ip:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    movl $16, -24(%ebp)\n");
    out.push_str("    lea -24(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    lea -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call getpeername\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jne .L_x86_peer_ip_empty\n");
    out.push_str("    push -16(%ebp)\n");
    out.push_str("    call inet_ntoa\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_peer_ip_empty\n");
    out.push_str("    mov %eax, %esi\n");
    emit_str_buf_load(out, "%ecx", "%edi", os);
    out.push_str("    cmp $1000000, %edi\n");
    out.push_str("    jl .L_x86_peer_ip_buf_ok\n");
    out.push_str("    xor %edi, %edi\n");
    out.push_str(".L_x86_peer_ip_buf_ok:\n");
    out.push_str("    lea (%ecx, %edi), %edx\n");
    out.push_str("    mov %edx, %ebx\n");
    out.push_str(".L_x86_peer_ip_copy:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    movb %al, (%edx)\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_peer_ip_copy_done\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edx\n");
    out.push_str("    jmp .L_x86_peer_ip_copy\n");
    out.push_str(".L_x86_peer_ip_copy_done:\n");
    out.push_str("    inc %edx\n");
    out.push_str("    sub %ecx, %edx\n");
    out.push_str("    add $3, %edx\n");
    out.push_str("    and $-4, %edx\n");
    emit_str_buf_store(out, "%edx", "%eax", os);
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    jmp .L_x86_peer_ip_ret\n");
    out.push_str(".L_x86_peer_ip_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_peer_ip_ret:\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_port: net_peer_port(sock) -> port or -1
    out.push_str(".global fn_net_peer_port\n");
    out.push_str("fn_net_peer_port:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $24, %esp\n");
    out.push_str("    movl $16, -24(%ebp)\n");
    out.push_str("    lea -24(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    lea -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call getpeername\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jne .L_x86_peer_port_fail\n");
    out.push_str("    movzwl -18(%ebp), %eax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    jmp .L_x86_peer_port_ret\n");
    out.push_str(".L_x86_peer_port_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_peer_port_ret:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_socket: creates a UDP socket (2, 2, 0)
    out.push_str(".global fn_net_udp_socket\n");
    out.push_str("fn_net_udp_socket:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push $0\n");
    out.push_str("    push $2\n");
    out.push_str("    push $2\n");
    out.push_str("    call socket\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jge .L_x86_udp_socket_ok\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_udp_socket_ok:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_bind: binds UDP socket to port -> 0 or -1
    out.push_str(".global fn_net_udp_bind\n");
    out.push_str("fn_net_udp_bind:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $20, %esp\n");
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str("    movl $0, -12(%ebp)\n");
    out.push_str("    movl $0, -8(%ebp)\n");
    out.push_str("    movw $2, -20(%ebp)\n");
    out.push_str("    mov 12(%ebp), %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, -18(%ebp)\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str("    push $16\n");
    out.push_str("    lea -20(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call bind\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jl .L_x86_udp_bind_fail\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    jmp .L_x86_udp_bind_ret\n");
    out.push_str(".L_x86_udp_bind_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_udp_bind_ret:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_send: net_udp_send(sock, host, port, data_str) -> bytes_sent or -1
    out.push_str(".global fn_net_udp_send\n");
    out.push_str("fn_net_udp_send:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $28, %esp\n");
    out.push_str("    mov 12(%ebp), %esi\n"); // host
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_usend_fail\n");
    out.push_str("    movl $0, -28(%ebp)\n");
    out.push_str("    movl $0, -24(%ebp)\n");
    out.push_str("    movl $0, -20(%ebp)\n");
    out.push_str("    movl $0, -16(%ebp)\n");
    out.push_str("    movw $2, -28(%ebp)\n");
    out.push_str("    mov 16(%ebp), %ax\n");
    out.push_str("    xchg %al, %ah\n");
    out.push_str("    mov %ax, -26(%ebp)\n");
    out.push_str("    push %esi\n");
    out.push_str("    call inet_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    cmp $0xffffffff, %eax\n");
    out.push_str("    jne .L_x86_usend_have_ip\n");
    out.push_str("    push %esi\n");
    out.push_str("    call gethostbyname\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usend_fail\n");
    out.push_str("    mov 16(%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usend_fail\n");
    out.push_str("    mov (%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_usend_fail\n");
    out.push_str("    movl (%eax), %eax\n");
    out.push_str(".L_x86_usend_have_ip:\n");
    out.push_str("    movl %eax, -24(%ebp)\n");
    out.push_str("    mov 20(%ebp), %esi\n"); // data_str
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_usend_call\n");
    out.push_str("    mov %esi, %edi\n");
    out.push_str(".L_x86_usend_len:\n");
    out.push_str("    cmpb $0, (%edi)\n");
    out.push_str("    je .L_x86_usend_call\n");
    out.push_str("    inc %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_usend_len\n");
    out.push_str(".L_x86_usend_call:\n");
    out.push_str("    push $16\n");
    out.push_str("    lea -28(%ebp), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call sendto\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    jmp .L_x86_usend_ret\n");
    out.push_str(".L_x86_usend_fail:\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str(".L_x86_usend_ret:\n");
    out.push_str("    add $28, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_recv: net_udp_recv(sock, max_bytes) -> string
    out.push_str(".global fn_net_udp_recv\n");
    out.push_str("fn_net_udp_recv:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");  // sock
    out.push_str("    mov 12(%ebp), %esi\n"); // max_bytes
    out.push_str("    cmp $0, %esi\n");
    out.push_str("    jg .L_x86_urecv_chk\n");
    out.push_str("    mov $4096, %esi\n");
    out.push_str(".L_x86_urecv_chk:\n");
    out.push_str("    cmp $524288, %esi\n");
    out.push_str("    jle .L_x86_urecv_alloc\n");
    out.push_str("    mov $524288, %esi\n");
    out.push_str(".L_x86_urecv_alloc:\n");
    emit_str_buf_load(out, "%edx", "%edi", os);
    out.push_str("    mov $1000000, %ecx\n");
    out.push_str("    sub %esi, %ecx\n");
    out.push_str("    cmp %ecx, %edi\n");
    out.push_str("    jl .L_x86_urecv_buf_ok\n");
    out.push_str("    xor %edi, %edi\n");
    out.push_str(".L_x86_urecv_buf_ok:\n");
    out.push_str("    add %edi, %edx\n");
    out.push_str("    mov %edx, -4(%ebp)\n");
    out.push_str("    push $0\n"); // addrlen = NULL
    out.push_str("    push $0\n"); // src_addr = NULL
    out.push_str("    push $0\n"); // flags = 0
    out.push_str("    push %esi\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call recvfrom\n");
    out.push_str("    add $24, %esp\n");
    out.push_str("    cmp $0, %eax\n");
    out.push_str("    jle .L_x86_urecv_empty\n");
    out.push_str("    mov -4(%ebp), %edx\n");
    out.push_str("    movb $0, (%edx, %eax)\n");
    out.push_str("    lea 1(%eax, %edi), %ecx\n");
    out.push_str("    add $3, %ecx\n");
    out.push_str("    and $-4, %ecx\n");
    emit_str_buf_store(out, "%ecx", "%esi", os);
    out.push_str("    mov %edx, %eax\n");
    out.push_str("    jmp .L_x86_urecv_done\n");
    out.push_str(".L_x86_urecv_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_urecv_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_set_nonblocking: net_set_nonblocking(sock, mode) -> 0 or -1
    out.push_str(".global fn_net_set_nonblocking\n");
    out.push_str("fn_net_set_nonblocking:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    sub $8, %esp\n");
        out.push_str("    movl 12(%ebp), %eax\n"); // mode
        out.push_str("    movl %eax, -8(%ebp)\n");
        out.push_str("    leal -8(%ebp), %eax\n");
        out.push_str("    push %eax\n");           // arg3: &mode
        out.push_str("    push $0x8004667e\n");    // arg2: FIONBIO
        out.push_str("    push 8(%ebp)\n");        // arg1: sock
        out.push_str("    call ioctlsocket\n");
        out.push_str("    add $12, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jge .L_x86_snb_ok\n");
        out.push_str("    mov $-1, %eax\n");
        out.push_str("    jmp .L_x86_snb_ret\n");
        out.push_str(".L_x86_snb_ok:\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str(".L_x86_snb_ret:\n");
        out.push_str("    add $8, %esp\n");
    } else {
        out.push_str("    push %ebx\n");
        out.push_str("    push %esi\n");
        out.push_str("    mov 8(%ebp), %ebx\n");  // sock
        out.push_str("    mov 12(%ebp), %esi\n"); // mode
        out.push_str("    push $0\n");
        out.push_str("    push $3\n");            // F_GETFL = 3
        out.push_str("    push %ebx\n");
        out.push_str("    call fcntl\n");
        out.push_str("    add $12, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_snb_err\n");
        out.push_str("    test %esi, %esi\n");
        out.push_str("    jz .L_x86_snb_clear\n");
        out.push_str("    or $2048, %eax\n");     // O_NONBLOCK = 2048
        out.push_str("    jmp .L_x86_snb_set\n");
        out.push_str(".L_x86_snb_clear:\n");
        out.push_str("    and $-2049, %eax\n");
        out.push_str(".L_x86_snb_set:\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $4\n");            // F_SETFL = 4
        out.push_str("    push %ebx\n");
        out.push_str("    call fcntl\n");
        out.push_str("    add $12, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_snb_err\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str("    jmp .L_x86_snb_done\n");
        out.push_str(".L_x86_snb_err:\n");
        out.push_str("    mov $-1, %eax\n");
        out.push_str(".L_x86_snb_done:\n");
        out.push_str("    pop %esi\n");
        out.push_str("    pop %ebx\n");
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_net_poll: net_poll(sock, timeout_ms) -> 1 (ready), 0 (timeout), -1 (error)
    out.push_str(".global fn_net_poll\n");
    out.push_str("fn_net_poll:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    sub $280, %esp\n");
        out.push_str("    movl $1, -276(%ebp)\n"); // fd_count = 1
        out.push_str("    mov 8(%ebp), %eax\n");
        out.push_str("    movl %eax, -272(%ebp)\n"); // fd_array[0] = sock

        out.push_str("    mov 12(%ebp), %eax\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_poll_win_inf\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    movl %eax, -8(%ebp)\n");  // tv_sec
        out.push_str("    imul $1000, %edx, %edx\n");
        out.push_str("    movl %edx, -4(%ebp)\n");  // tv_usec
        out.push_str("    leal -8(%ebp), %eax\n");
        out.push_str("    push %eax\n");            // arg5: timeout
        out.push_str("    jmp .L_x86_poll_win_call\n");
        out.push_str(".L_x86_poll_win_inf:\n");
        out.push_str("    push $0\n");
        out.push_str(".L_x86_poll_win_call:\n");
        out.push_str("    push $0\n");              // arg4: exceptfds
        out.push_str("    push $0\n");              // arg3: writefds
        out.push_str("    leal -276(%ebp), %eax\n");
        out.push_str("    push %eax\n");            // arg2: readfds
        out.push_str("    push $0\n");              // arg1: nfds (ignored on Win)
        out.push_str("    call select\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jg .L_x86_poll_win_ready\n");
        out.push_str("    jl .L_x86_poll_win_err\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str("    jmp .L_x86_poll_win_done\n");
        out.push_str(".L_x86_poll_win_ready:\n");
        out.push_str("    mov $1, %eax\n");
        out.push_str("    jmp .L_x86_poll_win_done\n");
        out.push_str(".L_x86_poll_win_err:\n");
        out.push_str("    mov $-1, %eax\n");
        out.push_str(".L_x86_poll_win_done:\n");
        out.push_str("    add $280, %esp\n");
    } else {
        out.push_str("    push %ebx\n");
        out.push_str("    push %edi\n");
        out.push_str("    sub $144, %esp\n");
        out.push_str("    leal -136(%ebp), %edi\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str("    mov $32, %ecx\n");
        out.push_str("    rep stosl\n");

        out.push_str("    mov 8(%ebp), %eax\n"); // sock
        out.push_str("    mov %eax, %ecx\n");
        out.push_str("    shr $5, %eax\n");
        out.push_str("    and $31, %ecx\n");
        out.push_str("    bts %ecx, -136(%ebp, %eax, 4)\n");

        out.push_str("    mov 12(%ebp), %eax\n"); // timeout_ms
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jl .L_x86_poll_posix_inf\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    movl %eax, -144(%ebp)\n");
        out.push_str("    imul $1000, %edx, %edx\n");
        out.push_str("    movl %edx, -140(%ebp)\n");
        out.push_str("    leal -144(%ebp), %edx\n");
        out.push_str("    push %edx\n");
        out.push_str("    jmp .L_x86_poll_posix_call\n");
        out.push_str(".L_x86_poll_posix_inf:\n");
        out.push_str("    push $0\n");
        out.push_str(".L_x86_poll_posix_call:\n");
        out.push_str("    push $0\n");
        out.push_str("    push $0\n");
        out.push_str("    leal -136(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    mov 8(%ebp), %eax\n");
        out.push_str("    inc %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call select\n");
        out.push_str("    add $20, %esp\n");
        out.push_str("    cmp $0, %eax\n");
        out.push_str("    jg .L_x86_poll_posix_ready\n");
        out.push_str("    jl .L_x86_poll_posix_err\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str("    jmp .L_x86_poll_posix_done\n");
        out.push_str(".L_x86_poll_posix_ready:\n");
        out.push_str("    mov $1, %eax\n");
        out.push_str("    jmp .L_x86_poll_posix_done\n");
        out.push_str(".L_x86_poll_posix_err:\n");
        out.push_str("    mov $-1, %eax\n");
        out.push_str(".L_x86_poll_posix_done:\n");
        out.push_str("    add $144, %esp\n");
        out.push_str("    pop %edi\n");
        out.push_str("    pop %ebx\n");
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
