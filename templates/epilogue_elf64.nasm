; Assemble using: nasm -f bin epilogue_elf64.nasm -o epilogue_elf64.template
[bits 64]
mov rax, 60
xor rdi, rdi
syscall