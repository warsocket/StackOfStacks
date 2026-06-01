; Assemble with: nasm -f bin prologue_pe64.nasm -o prologue_pe64.template
[bits 64]

; --- DOS HEADER ---
db 'MZ'
times 58 db 0
dd 0x00000040           ; PE Header start EXACT op byte 64 (0x40)

; --- PE HEADER (Start op 0x40) ---
db 'PE', 0, 0
dw 0x8664               ; Machine: x86_64
dw 1                    ; 1 sectie
dd 0, 0, 0              ; TimeDate, Symbols ptr, Number of symbols
dw 240                  ; Grootte van Optional Header (Altijd 240 voor PE32+)
dw 0x0022               ; Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

; --- OPTIONAL HEADER (Start op 0x58) ---
dw 0x020B               ; PE32+ Magic
db 0, 0                 ; Linker versie

; --- VANAF HIER GAAN WE EXACTE OFFSETS PATCHEN IN RUST ---
; [Offset 0x60 vanaf start bestand]
dd 0                    ; Size of Code (Rust patcht dit!)
dd 0, 0                 ; Size of init/uninit data
dd 512                  ; Entry Point RVA (Geparkeerd op exact byte 512)
dd 512                  ; Base of Code

; --- WINDOWS CONFIGURATIE (Start op 0x78) ---
dq 0x00400000           ; Image Base
dd 0x00001000           ; Section Alignment (4096)
dd 0x00000200           ; File Alignment (512)
dw 4, 0, 0, 0, 4, 0     ; OS, Image, Subsystem versies
dd 0                    ; Win32 Value
; [Offset 0x98 vanaf start bestand]
dd 0                    ; Size of Image (Rust patcht dit!)
dd 512                  ; Size of Headers
dd 0                    ; CheckSum
dw 3                    ; Subsystem: 3 = Console
dw 0                    ; DllCharacteristics
dq 0x00100000, 0x00001000 ; Stack Reserve / Commit
dq 0x00100000, 0x00001000 ; Heap Reserve / Commit
dd 0, 16                ; Loader Flags, Number of Data Directories

; --- DATA DIRECTORIES (16 stuks * 8 bytes = 128 bytes leeg) ---
times 128 db 0

; --- SECTION HEADERS (Start op 0x148) ---
db '.text', 0, 0, 0     ; Exact 8 bytes ASCII naam (GEEN null-bytes ertussen!)
; [Offset 0x150 vanaf start bestand]
dd 0                    ; Virtual Size (Rust patcht dit!)
dd 0x00001000           ; Virtual Address (RVA)
; [Offset 0x158 vanaf start bestand]
dd 0                    ; Size of Raw Data (Rust patcht dit!)
dd 0x00000200           ; Pointer to Raw Data (512)
times 12 db 0           ; Overige velden leeg
dd 0xE0000020           ; CODE | EXECUTE | READ | WRITE (Sectie rechten)

; Vul de gehele header aan tot exact 512 bytes
times 512 - ($ - $$) db 0

; --- JOUW CODE START HIER (Byte 512 / Offset 0x200) ---
push rbp
mov rbp, rsp
sub rsp, 32             ; Shadow space voor Windows ABI

mov r12, -1
mov r13, -32            ; OPCODE_SIZE
