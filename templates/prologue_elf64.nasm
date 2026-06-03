; Assemble using: nasm -f bin prologue_elf64.nasm -o prologue_elf64.template
; This makes it easyter to create new template file than just a plain hex file you need to edit
; 32 bytes per instruction will be hardcoded, so if you pas that, you need to adjust at compiler (But you should not make it more big !)

; See https://upload.wikimedia.org/wikipedia/commons/e/e4/ELF_Executable_and_Linkable_Format_diagram_by_Ange_Albertini.png
[bits 64]

%define OPCODE_SIZE 32


; Virtual address of where the ELF file will be mapped
OFFSET: equ 0x10000

; ELF header
db 0x7f, "ELF" ; Magic number
db 2 ; class =  64 bit
db 1 ; little endian
db 1 ; version = 1
db 0 ; ABI
db 0 ; extended ABI
times 7 db 0 ; Padding
dw 2 ; type = executable
dw 0x3e ; arch = x86_64
dd 1 ; version = 1
dq entry_point + OFFSET ; Entry point in memmory

dq program_headers ; program header offset
dq 0 ; section_headers_start
dd 0 ; other flags
; Size of this header, 64 bytes.
dw program_headers; size of this header
; Size of a program header entry.
dw program_headers; size of program entry header
; Number of program header entries.
; dw 0 ; # of program entry headers
; dw 0x40-2 ; size of section entry header
; dw 0 ; # of section headers
; dw 0 ; index of section header with string table



program_headers:
dd 1 ; = loadable segment
dd 5; Flags: 0x01 = executable, 0x02 = writable, 0x04 = readable
dq 0 ; loadable segment offset (load everything from start)
dq OFFSET ; Virutal address where to palce this elf in memory
dq OFFSET ; Physical address where to palce this elf in memory (seems unsused in x86_64)
dq code_section_end ; size of segment in the elf file
dq code_section_end ; size of segment in memmory
; dq 0;0x200000 ; segment alignment (seems ignored)

; Program starts here
STACKSIZE equ 0x10000000; 256MB

; Actual code
entry_point:

; Setup 36 bytes
mov rax, 12 	; brk
xor rdi, rdi 	; 0 = get current
syscall 		; rax = current

mov rbp, STACKSIZE
add rbp, rax

mov rdi, STACKSIZE*2
add rdi, rax	; rax offset + rdi
mov rax, 12 	; brk
syscall

mov rsp, rax ; now RSP points to active stack and RBP to the inactive one
;load magic numbers
mov r12, -1
mov r13, -OPCODE_SIZE ;mask


call .get_rip
.get_rip:
pop r15
jmp short .functions_end

.read:
push 0
xor rax, rax
xor rdi, rdi
mov rsi, rsp
mov rdx, 1
syscall

mov rdx, -1
mov rcx, rax ; copy reutrn
pop rax ; char in rax
test rcx, rcx ; return == 0
cmovz rax, rdx ; -1 in rax if eof

pop r11 ;ret addr in r11
push rax ; char / -1 to stack
jmp r11


.write:
;fix annoydiing stack frame flipping ret addr and  parameter
pop r9	; ret
pop r8  ; the value
push r9
push r8

mov rax, 1
mov rsi, rsp
mov rdi, rax
mov rdx, rax
lea rsp, [rsp+8]
syscall
ret


.exit:
;Exit syscall code here
push 60
pop rax
xor edi, edi
syscall

.functions_end:
mov r12, r15
mov r13, r15
mov r14, r15
add r12, (.read - .get_rip)
add r13, (.write - .get_rip)
add r14, (.exit - .get_rip)


code_section_end:

align OPCODE_SIZE, db 0x90