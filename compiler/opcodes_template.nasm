; Assemble using: nasm -f bin opcodes_template.nasm -o opcodes.template
[bits 64]
org 0x00

%define OPCODE_SIZE 32
%define OPCODE_BITS 5; 32 is 5 bits aka  1 1111

%macro OPCODE_START 0
%endmacro

%macro OPCODE_END 0
    align 32, db 0x90
%endmacro

; ! PUSH -1
OPCODE_START
push -1
OPCODE_END

; ^ XOR
OPCODE_START
pop rcx
xor [rsp], rcx
OPCODE_END

; | OR
OPCODE_START
pop rcx
or [rsp], rcx
OPCODE_END

; & AND
OPCODE_START
pop rcx
and [rsp], rcx
OPCODE_END

; + ADD
OPCODE_START
pop rcx
add [rsp], rcx
OPCODE_END

; - SUB
OPCODE_START
pop rcx
sub [rsp], rcx
OPCODE_END

; * MUL
OPCODE_START
pop rax
pop rcx
mul rcx
push rax
OPCODE_END

; / DIV
OPCODE_START
xor rdx, rdx
pop rcx
pop rax
test rcx, rcx
jz short .div_zero ;jumps over div rcx
div rcx
.div_zero:
cmovz rax, rdx ; result  = 0 if division by zero
push rax
OPCODE_END

; $ STACK SWAP
OPCODE_START
xchg rbp, rsp
OPCODE_END

; ~ XCHANGE
OPCODE_START
pop rax
xchg rax, [rbp]
push rax
OPCODE_END

; = DUP
OPCODE_START
mov rax, [rsp]
push rax
OPCODE_END

; = JMP
OPCODE_START

; By the skin on our teeth, boy we barely made the 32 bytes
pop rax
; cmp rax, r12 ; r12=-1
inc rax ; -1 becomes 0 zo jz workls, but also, we added the 1 that takes aour calc ot the start of the next opcode.  2 for 1! 
jz short .jmp_exit

call .next ; stack = rip
; lea rbx, [rel $] ; rbx = rip
.next:
pop rbx

shl rax, OPCODE_BITS
and rbx, r13 ; r13 = -OPCODE_SIZE bitmask to floor our offset

add rbx, rax
jmp rbx

.jmp_exit:
jmp r14 ; To sys_exit syscall

OPCODE_END

; ? READ
OPCODE_START
push 0
xor rax, rax
xor rdi, rdi
mov rsi, rsp
mov rdx, 1
syscall

mov rcx, rax ; copy reutrn
pop rax ; char in rax
test rcx, rcx ; return == 0
cmovz rax, r12 ; -1 in rax if eof
push rax ; char / -1 to stack

OPCODE_END

; . WRITE
OPCODE_START
mov rax, 1
mov rsi, rsp
mov rdi, rax
mov rdx, rax
lea rsp, [rsp+8]
syscall
OPCODE_END

; 0 SHL0
OPCODE_START
pop rax
shl rax, 1
push rax
OPCODE_END

; 1 SHL1
OPCODE_START
pop rax
shl rax, 1
or al, 1
push rax
OPCODE_END