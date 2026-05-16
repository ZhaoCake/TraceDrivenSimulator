#include "trace.h"

/// 将原始的 u8 值转换为 OpType 枚举
/// 用于解析二进制 Trace 文件中的指令类型字段
OpType op_type_from_u8(uint8_t val) {
    switch (val) {
        case 1: return OpType::IntAlu;
        case 2: return OpType::Load;
        case 3: return OpType::Store;
        case 4: return OpType::Branch;
        case 5: return OpType::Jump;
        case 6: return OpType::System;
        default: return OpType::Unknown;
    }
}

/// 将从文件直接映射出的 raw record 转换为结构化的 TraceRecord
TraceRecord TraceRecord::from_raw(const TraceRecordRaw& raw) {
    return TraceRecord{
        raw.pc,
        raw.inst,
        raw.rs1,
        raw.rs2,
        raw.rd,
        op_type_from_u8(raw.op_type),
        raw.mem_addr,
        raw.rd_val,
    };
}
