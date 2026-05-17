#include <iostream>
#include <iomanip>
#include "simulator/simulator.h"
#include "simulator/alu.h"
#include "simulator/hazard.h"

// ── 构造与初始化 ──

Simulator::Simulator()
    : cycle(0)
    , inst_count(0)
    , op_stats()
    , stall_count(0)
    , forward_count(0)
    , fd_latch_(std::nullopt)
    , de_latch_(std::nullopt)
    , em_latch_(std::nullopt)
    , mw_latch_(std::nullopt)
    , lsu_()
    , registers_()     // 默认零初始化（x0 为 0，符合 RISC-V 规范）
    , trace_()
    , stall_(false)
{
    registers_.fill(0);
}

PipelineSnapshot Simulator::snapshot() const {
    return PipelineSnapshot(
        fd_latch_,
        de_latch_,
        em_latch_,
        mw_latch_,
        registers_,
        cycle
    );
}

// ── Trace 加载 ──

void Simulator::load_trace(const std::vector<TraceRecord>& records) {
    trace_.assign(records.begin(), records.end());
}

// ── 流水线状态查询 ──

bool Simulator::is_done() const {
    return trace_.empty()
        && !fd_latch_.has_value()
        && !de_latch_.has_value()
        && !em_latch_.has_value()
        && !mw_latch_.has_value();
}

// ── 两相周期推进 ──

void Simulator::step_cycle() {
    // ── 第一相（快照）──
    PipelineSnapshot snap = snapshot();

    // ── 第二相（组合逻辑）──
    HazardResult hazard = HazardUnit::resolve(snap);

    // ── 第三相（时序逻辑）：反向提交管理锁存器转移 ──

    // WB：将 MEM/WB 锁存器的结果写回寄存器文件。
    // 使用快照（克隆），因为 WB 读取不可变视图。
    if (snap.mw.has_value()) {
        const auto& mw = snap.mw.value();
        if (mw.record.rd != 0) {
            uint64_t value = (mw.record.op_type == OpType::Load)
                ? mw.mem_result
                : mw.alu_result;
            registers_[mw.record.rd] = value;
        }
        inst_count += 1;
        op_stats[mw.record.op_type] += 1;
    }

    // MEM：推进 EX/MEM → MEM/WB。Load 从 Trace 中获取访存结果。
    if (em_latch_.has_value()) {
        auto em = std::move(em_latch_.value());
        uint64_t mem_result = (em.record.op_type == OpType::Load)
            ? em.record.rd_val
            : 0;
        mw_latch_ = MemWbLatch(em.record, em.alu_result, mem_result);
        em_latch_ = std::nullopt;
    } else {
        mw_latch_ = std::nullopt;
    }

    // EX：推进 ID/EX → EX/MEM。使用 Alu::compute 进行结果路由。
    if (de_latch_.has_value()) {
        auto de = std::move(de_latch_.value());
        uint64_t alu_result = Alu::compute(de.record, de.rs1_val, de.rs2_val);
        em_latch_ = ExMemLatch(de.record, alu_result);
        de_latch_ = std::nullopt;
    } else {
        em_latch_ = std::nullopt;
    }

    // ID：推进 IF/ID → ID/EX（含转发），应用 HazardResult。
    if (hazard.stall_id || hazard.stall_if) {
        // Load-Use 冒险：冻结 FD 锁存器，向 DE 插入气泡。
        stall_ = true;
        stall_count += 1;
        de_latch_ = std::nullopt;
        // fd_latch 保持冻结（不消耗）
    } else {
        // 清除上一周期的停顿标志。
        stall_ = false;
        // 推进 fd → de（操作数来自 HazardUnit 解析的转发源）。
        if (fd_latch_.has_value()) {
            auto fd = std::move(fd_latch_.value());
            uint64_t rs1_val = hazard.fwd_src1.get_value();
            uint64_t rs2_val = hazard.fwd_src2.get_value();
            de_latch_ = IdExLatch(fd.record, rs1_val, rs2_val);
            fd_latch_ = std::nullopt;
            if (hazard.fwd_src1.is_forward()) forward_count += 1;
            if (hazard.fwd_src2.is_forward()) forward_count += 1;
        } else {
            de_latch_ = std::nullopt;
        }
    }

    // IF：从 Trace 队列取指。
    if (!stall_ && !fd_latch_.has_value()) {
        if (!trace_.empty()) {
            fd_latch_ = IfIdLatch(trace_.front());
            trace_.pop_front();
        }
    }

    cycle += 1;
}

// ── 统计打印 ──

/// 将 OpType 转为可读的中文字符串
static const char* op_type_name(OpType op) {
    switch (op) {
        case OpType::Unknown: return "Unknown";
        case OpType::IntAlu:  return "IntAlu";
        case OpType::Load:    return "Load";
        case OpType::Store:   return "Store";
        case OpType::Branch:  return "Branch";
        case OpType::Jump:    return "Jump";
        case OpType::System:  return "System";
        default:              return "???";
    }
}

void Simulator::print_statistics() const {
    double ipc = (cycle > 0) ? (static_cast<double>(inst_count) / cycle) : 0.0;

    std::cout << "\n=========== 性能模拟统计结果 ===========" << std::endl;
    std::cout << "执行周期总计 (Trace Cycles) : " << cycle << std::endl;
    std::cout << "处理指令条数 (Inst Count)   : " << inst_count << std::endl;
    std::cout << std::fixed << std::setprecision(3);
    std::cout << "综合 IPC (Instr Per Cycle)  : " << ipc << std::endl;
    std::cout << "转发命中次数 (Forwards)     : " << forward_count << std::endl;
    std::cout << "Load-Use 停顿次数 (Stalls)  : " << stall_count << std::endl;
    std::cout << "-------- 基础指令分布 (Inst Mix) --------" << std::endl;
    for (const auto& [op, count] : op_stats) {
        double percentage = (static_cast<double>(count) / inst_count) * 100.0;
        std::cout << "  - " << std::left << std::setw(10) << op_type_name(op)
                  << ": " << std::right << std::setw(8) << count
                  << " 条 (" << std::fixed << std::setprecision(2)
                  << percentage << "%)" << std::endl;
    }
    std::cout << "========================================" << std::endl;
}
