# riscv-naive-assembler
A naive assembler for RISC-V, only some special instructions are supported: B-Extension.

# Usage
```text
riscv-naive-assembler --input in.S 
```
or
```text
cat in.S | riscv-naive-assembler
```
It can only convert RISC-V extension instructions into .byte instructions. Other instructions are dumped directly.


