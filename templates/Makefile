.PHONY: all clean

all: prologue.template opcodes.template epilogue.template
clean:
	rm -f *.template

%.template: %.nasm
	nasm -f bin $< -o $@
