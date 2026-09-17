use crate::codegen::target::OperatingSystem;
use super::{emit_adrp_add, emit_str_buf_ctx};

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_mac = matches!(os, OperatingSystem::MacOS);
    let p = if is_mac { "_" } else { "" };
    let _ = (is_mac, p);

    // fn_net_socket: creates a TCP socket (2, 1, 0)
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_socket\n");
    out.push_str("fn_net_socket:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x0, #2\n");
    out.push_str("    mov x1, #1\n");
    out.push_str("    mov x2, #0\n");
    out.push_str(&format!("    bl {}socket\n", p));
    out.push_str("    sxtw x0, w0\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    bge .L_arm64_socket_ok\n");
    out.push_str("    mvn x0, xzr\n"); // -1
    out.push_str(".L_arm64_socket_ok:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_net_connect: connect(host, port) -> socket or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_connect\n");
    out.push_str("fn_net_connect:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n"); // host
    out.push_str("    mov x20, x1\n"); // port
    out.push_str("    cbz x19, .L_arm64_conn_fail\n");

    // Clear sockaddr_in at [sp, #48] (16 bytes)
    out.push_str("    str xzr, [sp, #48]\n");
    out.push_str("    str xzr, [sp, #56]\n");
    if is_mac {
        out.push_str("    mov w2, #0x0210\n");
        out.push_str("    strh w2, [sp, #48]\n");
    } else {
        out.push_str("    mov w2, #2\n");
        out.push_str("    strh w2, [sp, #48]\n");
    }
    // htons(port)
    out.push_str("    rev16 w2, w20\n");
    out.push_str("    strh w2, [sp, #50]\n");

    // inet_addr(host)
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    bl {}inet_addr\n", p));
    out.push_str("    cmn w0, #1\n");
    out.push_str("    bne .L_arm64_conn_have_ip\n");

    // gethostbyname(host)
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    bl {}gethostbyname\n", p));
    out.push_str("    cbz x0, .L_arm64_conn_fail\n");
    out.push_str("    ldr x0, [x0, #24]\n"); // h_addr_list
    out.push_str("    cbz x0, .L_arm64_conn_fail\n");
    out.push_str("    ldr x0, [x0]\n");
    out.push_str("    cbz x0, .L_arm64_conn_fail\n");
    out.push_str("    ldr w0, [x0]\n");
    out.push_str(".L_arm64_conn_have_ip:\n");
    out.push_str("    str w0, [sp, #52]\n"); // sin_addr

    // socket(2, 1, 0)
    out.push_str("    mov x0, #2\n");
    out.push_str("    mov x1, #1\n");
    out.push_str("    mov x2, #0\n");
    out.push_str(&format!("    bl {}socket\n", p));
    out.push_str("    cmp x0, #0\n");
    out.push_str("    blt .L_arm64_conn_fail\n");
    out.push_str("    mov x21, x0\n"); // socket

    // connect(sock, &sin, 16)
    out.push_str("    mov x0, x21\n");
    out.push_str("    add x1, sp, #48\n");
    out.push_str("    mov x2, #16\n");
    out.push_str(&format!("    bl {}connect\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_conn_close\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    b .L_arm64_conn_ret\n");
    out.push_str(".L_arm64_conn_close:\n");
    out.push_str("    mov x0, x21\n");
    out.push_str(&format!("    bl {}close\n", p));
    out.push_str(".L_arm64_conn_fail:\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_conn_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // fn_net_listen: net_listen(port, backlog) -> socket or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_listen\n");
    out.push_str("fn_net_listen:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n"); // port
    out.push_str("    mov x20, x1\n"); // backlog
    out.push_str("    cmp x20, #0\n");
    out.push_str("    bgt .L_arm64_listen_bl_ok\n");
    out.push_str("    mov x20, #10\n");
    out.push_str(".L_arm64_listen_bl_ok:\n");

    // socket(2, 1, 0)
    out.push_str("    mov x0, #2\n");
    out.push_str("    mov x1, #1\n");
    out.push_str("    mov x2, #0\n");
    out.push_str(&format!("    bl {}socket\n", p));
    out.push_str("    sxtw x0, w0\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    blt .L_arm64_listen_fail\n");
    out.push_str("    mov x21, x0\n"); // socket

    // sockaddr_in at [sp, #48]
    out.push_str("    str xzr, [sp, #48]\n");
    out.push_str("    str xzr, [sp, #56]\n");
    if is_mac {
        out.push_str("    mov w2, #0x0210\n");
        out.push_str("    strh w2, [sp, #48]\n");
    } else {
        out.push_str("    mov w2, #2\n");
        out.push_str("    strh w2, [sp, #48]\n");
    }
    out.push_str("    rev16 w2, w19\n");
    out.push_str("    strh w2, [sp, #50]\n");
    out.push_str("    str wzr, [sp, #52]\n"); // INADDR_ANY

    // bind(sock, &sin, 16)
    out.push_str("    mov x0, x21\n");
    out.push_str("    add x1, sp, #48\n");
    out.push_str("    mov x2, #16\n");
    out.push_str(&format!("    bl {}bind\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_listen_close\n");

    // listen(sock, backlog)
    out.push_str("    mov x0, x21\n");
    out.push_str("    mov x1, x20\n");
    out.push_str(&format!("    bl {}listen\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_listen_close\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    b .L_arm64_listen_ret\n");
    out.push_str(".L_arm64_listen_close:\n");
    out.push_str("    mov x0, x21\n");
    out.push_str(&format!("    bl {}close\n", p));
    out.push_str(".L_arm64_listen_fail:\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_listen_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // fn_net_accept: net_accept(server_sock) -> client_sock or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_accept\n");
    out.push_str("fn_net_accept:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x1, #0\n");
    out.push_str("    mov x2, #0\n");
    out.push_str(&format!("    bl {}accept\n", p));
    out.push_str("    sxtw x0, w0\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    bge .L_arm64_accept_ok\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_accept_ok:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_net_send: net_send(sock, data_str) -> bytes_sent or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_send\n");
    out.push_str("fn_net_send:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n"); // sock
    out.push_str("    mov x20, x1\n"); // str
    out.push_str("    cbz x20, .L_arm64_send_zero\n");
    // strlen
    out.push_str("    mov x21, x20\n");
    out.push_str("    mov x22, #0\n");
    out.push_str(".L_arm64_send_len:\n");
    out.push_str("    ldrb w2, [x21]\n");
    out.push_str("    cbz w2, .L_arm64_send_do\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    add x22, x22, #1\n");
    out.push_str("    b .L_arm64_send_len\n");
    out.push_str(".L_arm64_send_do:\n");
    out.push_str("    cbz x22, .L_arm64_send_zero\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x22\n");
    out.push_str("    mov x3, #0\n");
    out.push_str(&format!("    bl {}send\n", p));
    out.push_str("    b .L_arm64_send_ret\n");
    out.push_str(".L_arm64_send_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_send_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_recv: net_recv(sock, max_bytes) -> string
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_recv\n");
    out.push_str("fn_net_recv:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n"); // sock
    out.push_str("    mov x20, x1\n"); // max_bytes
    out.push_str("    cmp x20, #0\n");
    out.push_str("    bgt .L_arm64_recv_chk\n");
    out.push_str("    mov x20, #4096\n");
    out.push_str(".L_arm64_recv_chk:\n");
    out.push_str("    movz x2, #8, lsl #16\n"); // 524288 (0x80000)
    out.push_str("    cmp x20, x2\n");
    out.push_str("    csel x20, x2, x20, gt\n");

    emit_str_buf_ctx(out, "x5", "x2", "x6", os);
    out.push_str("    ldr x3, [x2]\n");
    out.push_str("    movz x4, #16960\n");
    out.push_str("    movk x4, #15, lsl #16\n"); // 1000000 (0xF4240)
    out.push_str("    sub x4, x4, x20\n");
    out.push_str("    cmp x3, x4\n");
    out.push_str("    csel x3, xzr, x3, hi\n");
    out.push_str("    add x21, x5, x3\n"); // x21 = buffer
    out.push_str("    mov x22, x3\n");      // x22 = start idx

    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, x21\n");
    out.push_str("    mov x2, x20\n");
    out.push_str("    mov x3, #0\n");
    out.push_str(&format!("    bl {}recv\n", p));
    out.push_str("    cmp x0, #0\n");
    out.push_str("    ble .L_arm64_recv_empty\n");
    out.push_str("    strb wzr, [x21, x0]\n");
    emit_str_buf_ctx(out, "x5", "x2", "x6", os);
    out.push_str("    add x1, x22, x0\n");
    out.push_str("    add x1, x1, #8\n");
    out.push_str("    and x1, x1, #-8\n");
    out.push_str("    str x1, [x2]\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    b .L_arm64_recv_done\n");
    out.push_str(".L_arm64_recv_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_recv_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_close: net_close(sock) -> 0
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_close\n");
    out.push_str("fn_net_close:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(&format!("    bl {}close\n", p));
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_net_set_timeout: net_set_timeout(sock, ms) -> 0 or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_set_timeout\n");
    out.push_str("fn_net_set_timeout:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    mov x2, #1000\n");
    out.push_str("    udiv x3, x20, x2\n");
    out.push_str("    msub x4, x3, x2, x20\n");
    out.push_str("    mul x4, x4, x2\n");
    out.push_str("    stp x3, x4, [sp, #32]\n");
    let (sol, rcv_opt, snd_opt) = if is_mac {
        ("0xffff", "0x1006", "0x1005")
    } else {
        ("1", "20", "21")
    };
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    mov x1, #{}\n", sol));
    out.push_str(&format!("    mov x2, #{}\n", rcv_opt));
    out.push_str("    add x3, sp, #32\n");
    out.push_str("    mov x4, #16\n");
    out.push_str(&format!("    bl {}setsockopt\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_timeout_fail\n");
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    mov x1, #{}\n", sol));
    out.push_str(&format!("    mov x2, #{}\n", snd_opt));
    out.push_str("    add x3, sp, #32\n");
    out.push_str("    mov x4, #16\n");
    out.push_str(&format!("    bl {}setsockopt\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_timeout_fail\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    b .L_arm64_timeout_ret\n");
    out.push_str(".L_arm64_timeout_fail:\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_timeout_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_ip: net_peer_ip(sock) -> ip_string or ""
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_peer_ip\n");
    out.push_str("fn_net_peer_ip:\n");
    out.push_str("    stp x29, x30, [sp, #-64]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov w1, #16\n");
    out.push_str("    str w1, [sp, #48]\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    add x1, sp, #32\n");
    out.push_str("    add x2, sp, #48\n");
    out.push_str(&format!("    bl {}getpeername\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    bne .L_arm64_peer_ip_empty\n");
    out.push_str("    ldr w0, [sp, #36]\n");
    out.push_str(&format!("    bl {}inet_ntoa\n", p));
    out.push_str("    cbz x0, .L_arm64_peer_ip_empty\n");
    out.push_str("    mov x19, x0\n");
    emit_str_buf_ctx(out, "x4", "x1", "x6", os);
    out.push_str("    ldr x2, [x1]\n");
    out.push_str("    movz x3, #16960\n");
    out.push_str("    movk x3, #15, lsl #16\n"); // 1,000,000
    out.push_str("    cmp x2, x3\n");
    out.push_str("    csel x2, xzr, x2, hi\n");
    out.push_str("    add x20, x4, x2\n");
    out.push_str("    mov x5, x20\n");
    out.push_str("    mov x6, x19\n");
    out.push_str(".L_arm64_peer_ip_copy:\n");
    out.push_str("    ldrb w7, [x6], #1\n");
    out.push_str("    strb w7, [x5], #1\n");
    out.push_str("    cbnz w7, .L_arm64_peer_ip_copy\n");
    out.push_str("    sub x7, x5, x4\n");
    out.push_str("    add x7, x7, #8\n");
    out.push_str("    and x7, x7, #-8\n");
    out.push_str("    str x7, [x1]\n");
    out.push_str("    mov x0, x20\n");
    out.push_str("    b .L_arm64_peer_ip_ret\n");
    out.push_str(".L_arm64_peer_ip_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_peer_ip_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #64\n");
    out.push_str("    ret\n\n");

    // fn_net_peer_port: net_peer_port(sock) -> port or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_peer_port\n");
    out.push_str("fn_net_peer_port:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov w1, #16\n");
    out.push_str("    str w1, [sp, #40]\n");
    out.push_str("    add x1, sp, #24\n");
    out.push_str("    add x2, sp, #40\n");
    out.push_str(&format!("    bl {}getpeername\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    bne .L_arm64_peer_port_fail\n");
    out.push_str("    ldrh w0, [sp, #26]\n");
    out.push_str("    rev16 w0, w0\n");
    out.push_str("    b .L_arm64_peer_port_ret\n");
    out.push_str(".L_arm64_peer_port_fail:\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_peer_port_ret:\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_socket: creates a UDP socket (2, 2, 0)
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_udp_socket\n");
    out.push_str("fn_net_udp_socket:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x0, #2\n");
    out.push_str("    mov x1, #2\n");
    out.push_str("    mov x2, #0\n");
    out.push_str(&format!("    bl {}socket\n", p));
    out.push_str("    sxtw x0, w0\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    bge .L_arm64_udp_socket_ok\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_udp_socket_ok:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_bind: binds UDP socket to port -> 0 or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_udp_bind\n");
    out.push_str("fn_net_udp_bind:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    str xzr, [sp, #32]\n");
    out.push_str("    str xzr, [sp, #40]\n");
    if is_mac {
        out.push_str("    mov w2, #0x0210\n");
        out.push_str("    strh w2, [sp, #32]\n");
    } else {
        out.push_str("    mov w2, #2\n");
        out.push_str("    strh w2, [sp, #32]\n");
    }
    out.push_str("    rev16 w2, w20\n");
    out.push_str("    strh w2, [sp, #34]\n");
    out.push_str("    str wzr, [sp, #36]\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    add x1, sp, #32\n");
    out.push_str("    mov x2, #16\n");
    out.push_str(&format!("    bl {}bind\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    bge .L_arm64_ubind_ok\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str("    b .L_arm64_ubind_ret\n");
    out.push_str(".L_arm64_ubind_ok:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_ubind_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_send: net_udp_send(sock, host, port, data_str) -> bytes_sent or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_udp_send\n");
    out.push_str("fn_net_udp_send:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    mov x21, x2\n");
    out.push_str("    mov x22, x3\n");
    out.push_str("    cbz x20, .L_arm64_usend_fail\n");
    out.push_str("    str xzr, [sp, #48]\n");
    out.push_str("    str xzr, [sp, #56]\n");
    if is_mac {
        out.push_str("    mov w2, #0x0210\n");
        out.push_str("    strh w2, [sp, #48]\n");
    } else {
        out.push_str("    mov w2, #2\n");
        out.push_str("    strh w2, [sp, #48]\n");
    }
    out.push_str("    rev16 w2, w21\n");
    out.push_str("    strh w2, [sp, #50]\n");
    out.push_str("    mov x0, x20\n");
    out.push_str(&format!("    bl {}inet_addr\n", p));
    out.push_str("    cmn w0, #1\n");
    out.push_str("    bne .L_arm64_usend_have_ip\n");
    out.push_str("    mov x0, x20\n");
    out.push_str(&format!("    bl {}gethostbyname\n", p));
    out.push_str("    cbz x0, .L_arm64_usend_fail\n");
    out.push_str("    ldr x0, [x0, #24]\n");
    out.push_str("    cbz x0, .L_arm64_usend_fail\n");
    out.push_str("    ldr x0, [x0]\n");
    out.push_str("    cbz x0, .L_arm64_usend_fail\n");
    out.push_str("    ldr w0, [x0]\n");
    out.push_str(".L_arm64_usend_have_ip:\n");
    out.push_str("    str w0, [sp, #52]\n");
    out.push_str("    mov x1, x22\n");
    out.push_str("    mov x2, #0\n");
    out.push_str("    cbz x22, .L_arm64_usend_do\n");
    out.push_str(".L_arm64_usend_len:\n");
    out.push_str("    ldrb w3, [x1], #1\n");
    out.push_str("    cbz w3, .L_arm64_usend_do\n");
    out.push_str("    add x2, x2, #1\n");
    out.push_str("    b .L_arm64_usend_len\n");
    out.push_str(".L_arm64_usend_do:\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, x22\n");
    out.push_str("    mov x3, #0\n");
    out.push_str("    add x4, sp, #48\n");
    out.push_str("    mov x5, #16\n");
    out.push_str(&format!("    bl {}sendto\n", p));
    out.push_str("    b .L_arm64_usend_ret\n");
    out.push_str(".L_arm64_usend_fail:\n");
    out.push_str("    mvn x0, xzr\n");
    out.push_str(".L_arm64_usend_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // fn_net_udp_recv: net_udp_recv(sock, max_bytes) -> string
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_udp_recv\n");
    out.push_str("fn_net_udp_recv:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    cmp x20, #0\n");
    out.push_str("    bgt .L_arm64_urecv_chk\n");
    out.push_str("    mov x20, #4096\n");
    out.push_str(".L_arm64_urecv_chk:\n");
    out.push_str("    movz x2, #8, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    csel x20, x2, x20, gt\n");
    emit_str_buf_ctx(out, "x5", "x2", "x6", os);
    out.push_str("    ldr x3, [x2]\n");
    out.push_str("    movz x4, #16960\n");
    out.push_str("    movk x4, #15, lsl #16\n");
    out.push_str("    sub x4, x4, x20\n");
    out.push_str("    cmp x3, x4\n");
    out.push_str("    csel x3, xzr, x3, hi\n");
    out.push_str("    add x21, x5, x3\n");
    out.push_str("    mov x22, x3\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, x21\n");
    out.push_str("    mov x2, x20\n");
    out.push_str("    mov x3, #0\n");
    out.push_str("    mov x4, #0\n");
    out.push_str("    mov x5, #0\n");
    out.push_str(&format!("    bl {}recvfrom\n", p));
    out.push_str("    cmp x0, #0\n");
    out.push_str("    ble .L_arm64_urecv_empty\n");
    out.push_str("    strb wzr, [x21, x0]\n");
    emit_str_buf_ctx(out, "x5", "x2", "x6", os);
    out.push_str("    add x1, x22, x0\n");
    out.push_str("    add x1, x1, #8\n");
    out.push_str("    and x1, x1, #-8\n");
    out.push_str("    str x1, [x2]\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    b .L_arm64_urecv_done\n");
    out.push_str(".L_arm64_urecv_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_urecv_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_net_set_nonblocking: net_set_nonblocking(sock, mode) -> 0 or -1
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_set_nonblocking\n");
    out.push_str("fn_net_set_nonblocking:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n"); // sock
    out.push_str("    mov x20, x1\n"); // mode

    // fcntl(sock, F_GETFL, 0)
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, #3\n"); // F_GETFL = 3
    out.push_str("    mov x2, #0\n");
    if is_mac {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str xzr, [sp]\n");
        out.push_str(&format!("    bl {}fcntl\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}fcntl\n", p));
    }
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_snb_err\n");

    let nonblock_flag = if is_mac { 4 } else { 2048 };
    out.push_str("    cbz x20, .L_arm64_snb_clear\n");
    out.push_str(&format!("    orr w2, w0, #{}\n", nonblock_flag));
    out.push_str("    b .L_arm64_snb_apply\n");
    out.push_str(".L_arm64_snb_clear:\n");
    out.push_str(&format!("    bic w2, w0, #{}\n", nonblock_flag));
    out.push_str(".L_arm64_snb_apply:\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, #4\n"); // F_SETFL = 4
    out.push_str("    uxtw x2, w2\n");
    if is_mac {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x2, [sp]\n");
        out.push_str(&format!("    bl {}fcntl\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}fcntl\n", p));
    }
    out.push_str("    cmp w0, #0\n");
    out.push_str("    blt .L_arm64_snb_err\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    b .L_arm64_snb_ret\n");
    out.push_str(".L_arm64_snb_err:\n");
    out.push_str("    mvn x0, xzr\n"); // -1
    out.push_str(".L_arm64_snb_ret:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_net_poll: net_poll(sock, timeout_ms) -> 1 (ready), 0 (timeout), -1 (error)
    out.push_str(".align 2\n");
    out.push_str(".global fn_net_poll\n");
    out.push_str("fn_net_poll:\n");
    out.push_str("    stp x29, x30, [sp, #-192]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n"); // sock
    out.push_str("    mov x20, x1\n"); // timeout_ms

    // Zero 128-byte fd_set at [sp, #48]
    out.push_str("    add x21, sp, #48\n");
    out.push_str("    stp xzr, xzr, [x21]\n");
    out.push_str("    stp xzr, xzr, [x21, #16]\n");
    out.push_str("    stp xzr, xzr, [x21, #32]\n");
    out.push_str("    stp xzr, xzr, [x21, #48]\n");
    out.push_str("    stp xzr, xzr, [x21, #64]\n");
    out.push_str("    stp xzr, xzr, [x21, #80]\n");
    out.push_str("    stp xzr, xzr, [x21, #96]\n");
    out.push_str("    stp xzr, xzr, [x21, #112]\n");

    // Set bit in fd_set
    out.push_str("    lsr x1, x19, #6\n"); // word = sock / 64
    out.push_str("    and x2, x19, #63\n"); // bit = sock % 64
    out.push_str("    mov x3, #1\n");
    out.push_str("    lsl x3, x3, x2\n");
    out.push_str("    add x4, x21, x1, lsl #3\n");
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    orr x5, x5, x3\n");
    out.push_str("    str x5, [x4]\n");

    // Setup timeval at [sp, #176]
    out.push_str("    cmp x20, #0\n");
    out.push_str("    blt .L_arm64_poll_inf\n");
    out.push_str("    mov x1, #1000\n");
    out.push_str("    sdiv x2, x20, x1\n"); // tv_sec = timeout_ms / 1000
    out.push_str("    msub x3, x2, x1, x20\n"); // rem_ms = timeout_ms - (sec * 1000)
    out.push_str("    mul x3, x3, x1\n"); // tv_usec = rem_ms * 1000
    out.push_str("    add x22, sp, #176\n");
    out.push_str("    stp x2, x3, [x22]\n");
    out.push_str("    mov x4, x22\n"); // timeout arg
    out.push_str("    b .L_arm64_poll_call\n");
    out.push_str(".L_arm64_poll_inf:\n");
    out.push_str("    mov x4, #0\n");
    out.push_str(".L_arm64_poll_call:\n");
    out.push_str("    add x0, x19, #1\n"); // nfds = sock + 1
    out.push_str("    mov x1, x21\n"); // readfds
    out.push_str("    mov x2, #0\n"); // writefds = NULL
    out.push_str("    mov x3, #0\n"); // exceptfds = NULL
    out.push_str(&format!("    bl {}select\n", p));
    out.push_str("    cmp w0, #0\n");
    out.push_str("    bgt .L_arm64_poll_ready\n");
    out.push_str("    blt .L_arm64_poll_err\n");
    out.push_str("    mov x0, #0\n"); // timeout
    out.push_str("    b .L_arm64_poll_done\n");
    out.push_str(".L_arm64_poll_ready:\n");
    out.push_str("    mov x0, #1\n"); // ready
    out.push_str("    b .L_arm64_poll_done\n");
    out.push_str(".L_arm64_poll_err:\n");
    out.push_str("    mvn x0, xzr\n"); // -1
    out.push_str(".L_arm64_poll_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #192\n");
    out.push_str("    ret\n\n");
}
