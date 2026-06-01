; ==============================================================================
; Fully standalone 64-bit Windows Executable (Exit Code 0)
; Assemble with: & "C:\Program Files\NASM\nasm.exe" -f bin exit0.nasm -o exit0.exe
; ==============================================================================
[bits 64]

; --- DOS HEADER ---
db 'MZ'                     ; Magic number
times 58 db 0               ; Pad up to the PE header pointer
dd pe_header                ; Points to the PE header offset (0x40)

; --- PE HEADER ---
align 4
pe_header:
db 'PE', 0, 0               ; PE Signature
dw 0x8664                   ; Machine: AMD64 (x86_64)
dw 1                        ; Section count: Exactly 1 section (.text)
dd 0, 0, 0                  ; TimeDate, Symbols ptr, Number of symbols (0)
dw 240                      ; Size of Optional Header (Always 240 for PE32+)
dw 0x0022                   ; Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

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
dw 6, 0                     ; Major/Minor OS Version (6.0 = Windows Vista up to Windows 11)
dw 0, 0                     ; Major/Minor Image Version
dw 6, 0                     ; Major/Minor Subsystem Version (6.0 = Console)
dd 0                        ; Win32 Version Value (Reserved, must be 0)
dd 0x00002000               ; Size of Image (Headers + section space inside RAM)
dd 512                      ; Size of Headers (The headers consume exactly 512 bytes on disk)
dd 0                        ; CheckSum
dw 3                        ; Subsystem: 3 = Windows Console (CLI)
dw 0                        ; DllCharacteristics
dq 0x00100000, 0x00001000   ; Stack Reserve / Commit (1MB)
dq 0x00100000, 0x00001000   ; Heap Reserve / Commit
dd 0, 16                    ; Loader Flags, Number of Data Directories
times 16 * 8 db 0           ; Data Directories table (128 bytes of zeros)

; --- SECTION HEADERS ---
db '.text', 0, 0, 0     ; Exactly 8 bytes ASCII section name
dd 512                  ; Virtual Size (Virtual space allocation inside RAM)
dd 0x00001000           ; Virtual Address (RVA where this section lands in memory)
dd 512                  ; Size of Raw Data (Must be EXACTLY 512 to match disk padding)
dd 0x00000200           ; Pointer to Raw Data (Code starts at file offset 512)
times 12 db 0           ; Unused fields (Relocations, linenumbers, etc.)
dd 0xE0000020           ; Section Characteristics: CODE | EXECUTE | READ | WRITE

; Pad headers to exactly 512 bytes so the code starts at the correct boundary
align 512, db 0

; ==============================================================================
; THE EXECUTABLE CODE (Starts exactly at file offset 512 / RVA 0x1000)
; ==============================================================================
entry_point:
    push rbp
    mov rbp, rsp
    sub rsp, 32         ; Allocate Shadow Space (Mandatory for Windows x64 ABI)

    ; Program exit execution:
    add rsp, 32         ; Restore the allocated shadow space
    pop rbp
    xor rax, rax        ; RAX = 0 (This sets our exit status code)
    ret                 ; Return control back to the Windows OS loader

; Pad the final file to the next File Alignment boundary
align 512, db 0
