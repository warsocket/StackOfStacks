.PHONY: all clean

all: opcodes.template prologue_elf64.template epilogue_elf64.template prologue_pe64.template epilogue_pe64.template
clean:
	rm -f *.template

%.template: %.nasm
	nasm -f bin $< -o $@
