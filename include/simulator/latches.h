#pragma once

#include <cstdint>
#include <optional>
#include <array>
#include "trace.h"

/// IF/ID 锁存器 — 仅包含指令记录（尚无动态值）
struct IfIdLatch {
    TraceRecord record;

    explicit IfIdLatch(const TraceRecord& rec) : record(rec) {}
};

/// ID/EX 锁存器 — 包含经转发网络解析的寄存器值
struct IdExLatch {
    TraceRecord record;
    uint64_t rs1_val;
    uint64_t rs2_val;

    IdExLatch(const TraceRecord& rec, uint64_t rs1, uint64_t rs2)
        : record(rec), rs1_val(rs1), rs2_val(rs2) {}
};

/// EX/MEM 锁存器 — 包含 ALU 计算结果
struct ExMemLatch {
    TraceRecord record;
    uint64_t alu_result;

    ExMemLatch(const TraceRecord& rec, uint64_t alu)
        : record(rec), alu_result(alu) {}
};

/// MEM/WB 锁存器 — 包含 ALU 结果与访存结果
struct MemWbLatch {
    TraceRecord record;
    uint64_t alu_result;
    uint64_t mem_result;

    MemWbLatch(const TraceRecord& rec, uint64_t alu, uint64_t mem)
        : record(rec), alu_result(alu), mem_result(mem) {}
};

/// 流水线各锁存器状态的不可变快照。
/// 供组合逻辑（HazardUnit、Alu、LSU）使用，使其操作于一致的流水线状态视图之上。
struct PipelineSnapshot {
    std::optional<IfIdLatch> fd;       ///< IF/ID 锁存器
    std::optional<IdExLatch> de;       ///< ID/EX 锁存器
    std::optional<ExMemLatch> em;      ///< EX/MEM 锁存器
    std::optional<MemWbLatch> mw;      ///< MEM/WB 锁存器
    std::array<uint64_t, 32> registers;///< 体系结构寄存器文件（32 个整数寄存器，x0 硬连线为 0）
    uint64_t cycle;                     ///< 当前周期号

    PipelineSnapshot(
        std::optional<IfIdLatch> f,
        std::optional<IdExLatch> d,
        std::optional<ExMemLatch> e,
        std::optional<MemWbLatch> m,
        const std::array<uint64_t, 32>& regs,
        uint64_t cyc
    )
        : fd(std::move(f))
        , de(std::move(d))
        , em(std::move(e))
        , mw(std::move(m))
        , registers(regs)
        , cycle(cyc)
    {}
};
