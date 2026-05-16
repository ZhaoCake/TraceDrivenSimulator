#pragma once

#include <cstdint>
#include "trace.h"  // 需要 TraceRecord、OpType

/// 组合 ALU（算术逻辑单元）
/// 
/// Trace 驱动模拟器中，ALU 不执行真实算术运算
/// （Trace 提供预计算结果）。它仅从 Trace 记录中路由正确的值
/// 到流水线锁存器。
///
/// 纯组合函数 — 无状态、无时钟。
class Alu {
public:
    /// 计算指令的 ALU 结果
    /// 
    /// 对于 Trace 驱动的模拟器，Trace 提供了真实值：
    ///   - Load/Store: mem_addr 是实际 CPU 计算的有效地址
    ///   - Jump: pc + 4 是返回地址（JAL 链接）
    ///   - 其他所有情况: rd_val 是 Trace 中的预计算结果
    ///
    /// rs1_val 和 rs2_val 参数为 API 兼容性保留
    /// （在真实 ALU 中会是转发的操作数值），但不被使用
    /// 因为 Trace 已经包含了结果。
    static uint64_t compute(const TraceRecord& record, uint64_t rs1_val, uint64_t rs2_val);
};
