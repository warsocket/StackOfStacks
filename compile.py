#!/usr/bin/env python3
import sys
from pathlib import Path

OPCODE_SIZE = 32

# Bytecode to linux x86_64 native compiler
print(f"Compiling bytecode (stdin) to elf64 (stdout)", file=sys.stderr)
print(f"DEPRECATED: please use the mian tool with --elf64 to compile native binaries", file=sys.stderr)

templates_dir = Path(__file__).resolve().parent / "templates"

with open(templates_dir / "opcodes.template", "rb") as f:
	opcodes_bytes = f.read()

if len(opcodes_bytes) != OPCODE_SIZE*16:
	print(f"Resource error: opcodes.template file corrupt. (size: {len(opcodes_bytes)} != {OPCODE_SIZE*16})", file=sys.stderr)
	exit(1)


with open(templates_dir / "prologue.template", "rb") as f:
	prologue_bytes = f.read()

with open(templates_dir / "epilogue.template", "rb") as f:
	epilogue_bytes = f.read()


# Chop up opcodes
opcodes = []
for i in range(16):
	opcodes.append(opcodes_bytes[i*OPCODE_SIZE:i*OPCODE_SIZE+OPCODE_SIZE])


#  EMIT
bytecode_file = sys.stdin.buffer.read()


sys.stdout.buffer.write(prologue_bytes)

padding = (OPCODE_SIZE-len(prologue_bytes) % OPCODE_SIZE) % OPCODE_SIZE
#write padding to align code:
sys.stdout.buffer.write(b'\x90'*padding)

# Now to emit from bytecode: High nibble first
# yes bytecode can have 1 extra opcode, but /care
for byte in bytecode_file:
	high = (byte & 0xF0) >> 4
	low = byte & 0xF

	for token in [high,low]:
		sys.stdout.buffer.write(opcodes[token])


sys.stdout.buffer.write(epilogue_bytes)