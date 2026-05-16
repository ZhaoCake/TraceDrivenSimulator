#include "simulator/alu.h"

uint64_t Alu::compute(const TraceRecord& record, uint64_t /*rs1_val*/, uint64_t /*rs2_val*/) {
    // 根据指令类型路由正确的值
    switch (record.op_type) {
        case OpType::Load:
        case OpType::Store:
            // 内存指令：ALU 计算有效地址。Trace 提供实际 CPU 使用的地址。
            return record.mem_addr;

        case OpType::Jump:
            // JAL / JALR：ALU 计算返回地址 (pc + 4)
            return record.pc + 4;

        default:
            // IntAlu、分支、系统：使用 Trace 提供的结果
            return record.rd_val;
    }
}
