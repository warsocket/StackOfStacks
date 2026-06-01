; Assemble using: nasm -f bin epilogue_pe64.nasm -o epilogue_pe64.template
[bits 64]
; Herstel de stack frame die we in de proloog hebben gemaakt
add rsp, 32
pop rbp

; Sluit netjes af. RAX bevat de return/exit code (bijv. 0)
xor rax, rax
ret
