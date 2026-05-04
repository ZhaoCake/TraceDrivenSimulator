RISCV_GCC = riscv32-unknown-linux-gnu-gcc
RISCV_OBJDUMP = riscv32-unknown-linux-gnu-objdump
CFLAGS = -nostdlib -nostartfiles -static -no-pie -march=rv32i -mabi=ilp32
LDFLAGS = -T baremetal.ld

all: test_c

test_c: crt0.S test_c.c baremetal.ld
	$(RISCV_GCC) $(CFLAGS) $(LDFLAGS) crt0.S test_c.c -o test_c

disasm: test_c
	$(RISCV_OBJDUMP) -d test_c

trace: test_c
	spike --isa=rv32i -l --log-commits test_c 2> spike.log
	./parse_spike_log.py spike.log test_c.trace

clean:
	rm -f test_c test_addi spike.log *.trace

.PHONY: all disasm trace clean
