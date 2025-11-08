# Simple dummy program that just exits
.section .text
.global _start
_start:
    # Write a message to stdout (fd 1)
    mov $1, %rax        # sys_write
    mov $1, %rdi        # fd = stdout
    lea message(%rip), %rsi  # buffer (position-independent)
    mov $message_len, %rdx  # count
    syscall
    
    # Exit
    mov $60, %rax       # sys_exit
    mov $0, %rdi        # exit code
    syscall

.section .rodata
message:
    .ascii "Hello from /bin/init!\n"
message_len = . - message

