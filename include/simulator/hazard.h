#pragma once

#include <cstdint>
#include "trace.h"
#include "simulator/latches.h"

/// 转发操作数值的来源类型
enum class ForwardSourceKind {
    RegFile,    ///< 来自寄存器文件
    ExResult,   ///< 来自同周期 EX 阶段预计算结果（de_latch 中非 Load 指令的 ALU 输出）
    ExMemAlu,   ///< 来自 EX/MEM 锁存器的 ALU 结果（上一周期结果，Load 除外）
    MemWbAlu,   ///< 来自 MEM/WB 锁存器的 ALU 结果
    MemWbMem,   ///< 来自 MEM/WB 锁存器的访存加载结果
};

/// 指定转发操作数值的来源，包含来源类型与具体数值。
/// 对应 Rust 中携带值的枚举: ForwardSource { RegFile(u64), ExMemAlu(u64), MemWbAlu(u64), MemWbMem(u64) }
struct ForwardSource {
    ForwardSourceKind kind;
    uint64_t value;       ///< 转发源提供的实际值

    /// 从转发源中提取实际值
    uint64_t get_value() const { return value; }

    /// 若为转发命中（非寄存器文件）则返回 true
    bool is_forward() const { return kind != ForwardSourceKind::RegFile; }

    // 静态工厂方法
    static ForwardSource reg_file(uint64_t v) { return {ForwardSourceKind::RegFile, v}; }
    static ForwardSource ex_result(uint64_t v) { return {ForwardSourceKind::ExResult, v}; }
    static ForwardSource ex_mem_alu(uint64_t v) { return {ForwardSourceKind::ExMemAlu, v}; }
    static ForwardSource mem_wb_alu(uint64_t v) { return {ForwardSourceKind::MemWbAlu, v}; }
    static ForwardSource mem_wb_mem(uint64_t v) { return {ForwardSourceKind::MemWbMem, v}; }

    bool operator==(const ForwardSource& other) const {
        return kind == other.kind && value == other.value;
    }
    bool operator!=(const ForwardSource& other) const { return !(*this == other); }
};

/// 组合冒险解析的结果。
/// 所有字段均为时序提交阶段的控制信号。
struct HazardResult {
    /// 冻结 IF/ID 锁存器（PC 不递增）
    bool stall_if = false;
    /// 冻结 ID/EX 锁存器（插入气泡 / NOP）
    bool stall_id = false;
    /// 冲刷 IF/ID 锁存器（如分支预测错误）
    bool flush_if = false;
    /// 冲刷 ID/EX 锁存器
    bool flush_id = false;
    /// ID 阶段指令的 rs1 转发源
    ForwardSource fwd_src1 = ForwardSource::reg_file(0);
    /// ID 阶段指令的 rs2 转发源
    ForwardSource fwd_src2 = ForwardSource::reg_file(0);
};

/// 组合冒险检测与转发解析单元。
///
/// 纯函数 — 无状态、无寄存器、无时钟。
/// 接收流水线状态快照，产出控制信号供时序提交阶段使用。
class HazardUnit {
public:
    /// 解析所有冒险并确定转发来源。
    ///
    /// 输入：不可变流水线快照（周期开始时的锁存器状态）。
    /// 输出：控制信号（停顿/冲刷/转发），供提交阶段使用。
    static HazardResult resolve(const PipelineSnapshot& snapshot);

private:
    /// 解析单个寄存器操作数的转发来源。
    ///
    /// 优先级（硬件转发多路选择器顺序）：
    ///   1. x0 → 恒为 0（硬连线）
    ///   2. ID/EX 预计算（同周期 EX→ID 转发，非 Load 指令的 ALU 结果）
    ///   3. EX/MEM 的 alu_result（上一周期结果，Load 除外 — 数据未就绪）
    ///   4. MEM/WB 的 mem_result（Load）或 alu_result（其他）
    ///   5. 寄存器文件（默认/回退）
    static ForwardSource resolve_forward(uint8_t rs, const PipelineSnapshot& snap);
};
