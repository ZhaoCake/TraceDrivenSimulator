#!/usr/bin/env python3
import sys
import struct
import re

# TraceRecord size = 8 (pc) + 4 (inst) + 1 (rs1) + 1 (rs2) + 1 (rd) + 1 (op_type) + 8 (mem_addr) + 8 (rd_val) = 32 bytes

OP_UNKNOWN = 0
OP_INTALU = 1
OP_LOAD = 2
OP_STORE = 3
OP_BRANCH = 4
OP_JUMP = 5
OP_SYSTEM = 6

def get_op_type(opcode):
    if opcode == 0x33 or opcode == 0x13 or opcode == 0x3b or opcode == 0x1b: # OP, OP-IMM, OPW, OP-IMMW
        return OP_INTALU
    if opcode == 0x37 or opcode == 0x17: # LUI, AUIPC
        return OP_INTALU
    if opcode == 0x03: # LOAD
        return OP_LOAD
    if opcode == 0x23: # STORE
        return OP_STORE
    if opcode == 0x63: # BRANCH
        return OP_BRANCH
    if opcode == 0x6f or opcode == 0x67: # JAL, JALR
        return OP_JUMP
    if opcode == 0x73: # SYSTEM
        return OP_SYSTEM
    return OP_UNKNOWN

def parse_log(log_path, bin_path):
    # Regex to match the instruction line: core   0: 0x0000000000001000 (0x00000297) auipc   t0, 0x0
    re_inst = re.compile(r'core\s+\d+:\s+(0x[0-9a-fA-F]+)\s+\((0x[0-9a-fA-F]+)\)\s+(.*)')
    # Regex to match the commit line: core   0: 3 0x0000000000001000 (0x00000297) x5  0x0000000000001000
    re_commit = re.compile(r'core\s+\d+:\s+3\s+(0x[0-9a-fA-F]+)\s+\((0x[0-9a-fA-F]+)\)(.*)')

    records = []

    with open(log_path, 'r') as f:
        lines = f.readlines()

    current_record = None

    for line in lines:
        m1 = re_inst.match(line)
        if m1:
            pc = int(m1.group(1), 16)
            inst = int(m1.group(2), 16)
            
            opcode = inst & 0x7F
            op_type = get_op_type(opcode)
            
            # extract rs1, rs2, rd correctly from bit fields 
            # (Note for pure trace, parsing from inst bits is simplest and reliable for RV32/64IM)
            # R-type: rd=11:7, funct3=14:12, rs1=19:15, rs2=24:20
            rd = (inst >> 7) & 0x1F
            rs1 = (inst >> 15) & 0x1F
            rs2 = (inst >> 20) & 0x1F
            
            # fix missing dependencies based on opcode format limits
            if op_type in [OP_JUMP]: # JAL doesn't have rs1/rs2, JALR has rs1
                if opcode == 0x6f: # JAL
                    rs1 = rs2 = 0
                elif opcode == 0x67: # JALR
                    rs2 = 0
            elif op_type == OP_STORE or op_type == OP_BRANCH: # Store/Branch: no rd
                rd = 0
            elif op_type in [OP_LOAD, OP_INTALU, OP_SYSTEM]: 
                if opcode in [0x37, 0x17]: # LUI, AUIPC (U-type: no rs1/rs2)
                    rs1 = rs2 = 0
                elif opcode in [0x13, 0x1b, 0x03, 0x73]: # I-type: no rs2
                    rs2 = 0

            current_record = {
                "pc": pc,
                "inst": inst,
                "rs1": rs1,
                "rs2": rs2,
                "rd": rd,
                "op_type": op_type,
                "mem_addr": 0,
                "rd_val": 0,
            }
            continue

        m2 = re_commit.match(line)
        if m2 and current_record:
            pc = int(m2.group(1), 16)
            if pc != current_record["pc"]:
                continue # mismatch
            
            rest = m2.group(3).strip()
            # rest could be "x11 0x0000000000001020", "mem 0x0000000080001000 0x00000001", "x5 ... mem ..."
            parts = rest.split()
            rd_val = 0
            mem_addr = 0
            
            i = 0
            while i < len(parts):
                if parts[i].startswith('x'):
                    i += 1
                    if i < len(parts):
                        rd_val = int(parts[i], 16)
                        i += 1
                elif parts[i] == 'mem':
                    i += 1
                    if i < len(parts):
                        mem_addr = int(parts[i], 16)
                        i += 1
                        # Could be another value after mem_addr for write data skip it
                        if i < len(parts) and parts[i].startswith('0x'):
                            i += 1
                else:
                    i += 1

            current_record["rd_val"] = rd_val
            current_record["mem_addr"] = mem_addr
            
            records.append(current_record)
            current_record = None

    print(f"Parsed {len(records)} instructions. Writing to {bin_path}...")
    
    with open(bin_path, 'wb') as f:
        for r in records:
            # Q: unsigned long long (8), I: unsigned int (4), B: unsigned char (1)
            # pack format: <Q I B B B B Q Q -> 8 + 4 + 1 + 1 + 1 + 1 + 8 + 8 = 32 bytes
            packed = struct.pack('<Q I B B B B Q Q', 
                r["pc"], r["inst"], r["rs1"], r["rs2"], r["rd"], r["op_type"], r["mem_addr"], r["rd_val"])
            f.write(packed)
    print("Done.")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: ./parse_spike_log.py <spike.log> <output.trace>")
        sys.exit(1)
    parse_log(sys.argv[1], sys.argv[2])
