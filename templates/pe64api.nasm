; ==============================================================================
; Standalone 64-bit Windows Executable - Dynamic PEB Template (16-Byte Aligned)
; Compile with: & "C:\Program Files\NASM\nasm.exe" -f bin win64_peb_io.nasm -o test.exe
; ==============================================================================
[bits 64]
default rel

; --- DOS HEADER ---
db 'MZ'
times 58 db 0
dd pe_header

; --- PE HEADER ---
align 4
pe_header:
db 'PE', 0, 0
dw 0x8664               ; Machine: AMD64
dw 1                    ; 1 single section (.text)
dd 0, 0, 0
dw 240                  ; Size of Optional Header
dw 0x0022               ; Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

; --- OPTIONAL HEADER (PE32+) ---
optional_header:
dw 0x020B                   ; Magic: PE32+ (64-bit Windows)
db 0, 0                     ; Linker version
dd 512                      ; Size of Code (Size of our .text section on disk)
dd 0, 0                     ; Size of init / uninit data
dd 0x00001000               ; Address of Entry Point (Relative Virtual Address / RVA)
dd 0x00001000               ; Base of Code (RVA where the .text section starts)

; --- WINDOWS CONFIGURATION ---
dq 0x00400000               ; Image Base (Default loading address in RAM)
dd 0x00001000               ; Section Alignment (Virtual memory pages are 4096 bytes)
dd 0x00000200               ; File Alignment (File sections on disk are 512 bytes)
dw 6, 0                 ; OS Version (6.0 = Windows 10/11)
dw 0, 0
dw 6, 0                 ; Subsystem Version (6.0)
dd 0
dd 0x00002000           ; Size of Image
dd 512                  ; Size of Headers
dd 0
dw 3                    ; Subsystem: 3 = Windows Console (CLI)
dw 0                    ; DllCharacteristics
dq 0x00100000, 0x00001000 ; Stack Reserve / Commit
dq 0x00100000, 0x00001000 ; Heap Reserve / Commit
dd 0, 16
times 16 * 8 db 0       ; Data Directories (Empty, no IAT!)

; --- SECTION HEADERS ---
db '.text', 0, 0, 0
dd 512                  ; Virtual Size
dd 0x00001000           ; Virtual Address
dd 512                  ; Size of Raw Data
dd 0x00000200           ; Pointer to Raw Data
times 12 db 0
dd 0xE0000020           ; CODE | EXECUTE | READ | WRITE

align 512, db 0

; ==============================================================================
; THE EXECUTABLE CODE (Starts exactly at file offset 512 / RVA 0x1000)
; ==============================================================================
entry_point:

    push rbp
    mov rbp, rsp
    sub rsp, 32         ; Allocate Shadow Space (Mandatory for Windows x64 ABI)

    ; STACK ALIGNMENT:
    ; Entering: RSP is 8-byte aligned.
    ; push rbp: RSP becomes 16-byte aligned.
    ; sub rsp, 80: 80 is a multiple of 16, keeping RSP perfectly 16-byte aligned!
    sub rsp, 80                 ; Allocate shadow space + local pointers

    ; --------------------------------------------------------------------------
    ; STEP 1: Locate Kernel32.dll Base Address via PEB
    ; --------------------------------------------------------------------------
    mov rax, [gs:0x60]          ; RAX = PEB address
    mov rax, [rax + 0x18]       ; RAX = PEB->Ldr
    mov rax, [rax + 0x20]       ; RAX = PEB->Ldr->InMemoryOrderModuleList (target.exe)
    mov rax, [rax]              ; RAX = 2nd module (ntdll.dll)
    mov rax, [rax]              ; RAX = 3rd module (kernel32.dll)
    mov rbx, [rax + 0x20]       ; RBX = Kernel32.dll BaseAddress (Offset 0x20 inside list node)
    mov [rbp - 8], rbx          ; Store Kernel32 base at [rbp - 8]

    ; --------------------------------------------------------------------------
    ; STEP 2: Dynamically Resolve API Function Addresses
    ; --------------------------------------------------------------------------
    lea rsi, [rel str_getstdhandle]
    call get_proc_address
    mov [rbp - 16], rax         ; Store GetStdHandle pointer at [rbp - 16]

    lea rsi, [rel str_writefile]
    call get_proc_address
    mov [rbp - 24], rax         ; Store WriteFile pointer at [rbp - 24]

    lea rsi, [rel str_exitprocess]
    call get_proc_address
    mov [rbp - 32], rax         ; Store ExitProcess pointer at [rbp - 32]

    ; --------------------------------------------------------------------------
    ; STEP 3: Write Character 'A' to stdout
    ; --------------------------------------------------------------------------
    ; 1. Get stdout handle
    mov rcx, -11                ; STD_OUTPUT_HANDLE = -11
    call [rbp - 16]             ; Call GetStdHandle
    mov rbx, rax                ; RBX = stdout handle

    ; 2. Put 'A' on local stack frame
    mov byte [rbp - 40], 0x41   ; ASCII 'A' at a safe offset

    ; 3. Call WriteFile
    mov rcx, rbx                ; 1st arg: stdout handle
    lea rdx, [rbp - 40]         ; 2nd arg: pointer to character 'A'
    mov r8, 1                   ; 3rd arg: 1 byte
    lea r9, [rbp - 48]          ; 4th arg: pointer to receive bytes written
    mov qword [rsp + 32], 0     ; 5th arg: lpOverlapped = NULL (on stack)
    call [rbp - 24]             ; Call WriteFile

    ; --------------------------------------------------------------------------
    ; STEP 4: Flush & Exit cleanly via ExitProcess
    ; --------------------------------------------------------------------------
    xor rcx, rcx                ; Exit code 0
    call [rbp - 32]             ; Call ExitProcess (Flushes buffers, never returns)




    ; ; Program exit execution:
    ; add rsp, 32         ; Restore the allocated shadow space
    ; pop rbp
    ; xor rax, rax        ; RAX = 0 (This sets our exit status code)
    ; ret                 ; Return control back to the Windows OS loader

    ; push rbp
    ; mov rbp, rsp

; ==============================================================================
; HELPER: GetProcAddress parser (Scans export table manually)
; Inputs: [rbp - 8] = DLL Base, RSI = Pointer to Null-Terminated String
; Outputs: RAX = Function Pointer
; ==============================================================================
get_proc_address:
    push rbx
    push rcx
    push rdx
    push rdi
    push rsi
    
    mov rbx, [rbp - 8]          ; Load Kernel32 Base Address
    mov edx, [rbx + 0x3c]       ; EDX = PE header offset
    add rdx, rbx                ; RDX = PE header address
    mov edx, [rdx + 0x88]       ; EDX = Export Table RVA
    add rdx, rbx                ; RDX = Export Table address
    
    push rdx                    ; Save Export Table Address to reuse later
    mov ecx, [rdx + 0x18]       ; ECX = Number of Names
    mov edi, [rdx + 0x20]       ; EDI = Address of Names RVA
    add rdi, rbx                ; EDI = Address of Names absolute address

.search_loop:
    dec ecx
    js .not_found
    mov esi, [rdi + rcx*4]      ; ESI = Name RVA
    add rsi, rbx                ; RSI = Name absolute address
    
    ; Compare string loop
    mov r10, [rsp + 8]          ; Restore original target string pointer (RSI) from stack
    xor r11, r11
.compare_bytes:
    mov al, [rsi + r11]
    mov dl, [r10 + r11]
    cmp al, dl
    jne .search_loop
    test al, al
    jnz .next_byte
    jmp .found
.next_byte:
    inc r11
    jmp .compare_bytes

.found:
    pop rdx                     ; Restore Export Table Address
    mov edi, [rdx + 0x24]       ; EDI = Ordinal Table RVA
    add rdi, rbx
    movzx ecx, word [rdi + rcx*2] ; ECX = Function Ordinal
    
    mov edi, [rdx + 0x1c]       ; EDI = Address Table RVA
    add rdi, rbx
    mov eax, [rdi + rcx*4]      ; EAX = Function RVA
    add rax, rbx                ; RAX = Absolute Address
    jmp .out

.not_found:
    pop rdx                     ; Clean up saved address from stack
    xor rax, rax
.out:
    pop rsi
    pop rdi
    pop rdx
    pop rcx
    pop rbx
    ret

; Null-terminated function strings
str_getstdhandle: db 'GetStdHandle', 0
str_writefile:    db 'WriteFile', 0
str_exitprocess:  db 'ExitProcess', 0

; Force the complete flat binary file to end exactly at the 1024-byte boundary
align 512, db 0
