; Assemble using: nasm -f bin epilogue.nasm -o epilogue.template
[bits 64]
mov rax, 60
xor rdi, rdi
syscall