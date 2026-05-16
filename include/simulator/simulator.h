#pragma once

#include <cstdint>
#include <optional>
#include <deque>
#include <unordered_map>
#include <array>
#include <vector>
#include "trace.h"
#include "simulator/latches.h"
#include "simulator/lsu.h"

/// 性能仿真器核心类
/// 实现经典五级流水线（IF/ID/EX/MEM/WB），包含转发网络与 Load-Use 冒险停顿
class Simulator {
public:
    /// 模拟推演至目前花费的周期总数
    uint64_t cycle = 0;
    /// 成功执行并提交的指令数目（在 WB 阶段计数）
    uint64_t inst_count = 0;
    /// 按照操作类别进行的分布计数器（指令混例统计）
    std::unordered_map<OpType, uint64_t> op_stats;

    // ── 公开统计计数器 ──
    /// Load-Use 冒险导致的停顿次数
    uint64_t stall_count = 0;
    /// 转发网络命中次数（从流水线锁存器获取操作数而非寄存器文件）
    uint64_t forward_count = 0;

    /// 构造一个新的空白模拟器对象
    Simulator();

    /// 获取当前流水线状态的不可变快照，供组合逻辑（HazardUnit、Alu、LSU）使用
    PipelineSnapshot snapshot() const;

    /// 将全部 Trace 记录加载到模拟器的输入队列
    void load_trace(const std::vector<TraceRecord>& records);

    /// 判断流水线是否已全部排空（无待处理指令，所有锁存器均为空）
    bool is_done() const;

    /// 两相周期推进：快照 → 组合逻辑 → 时序逻辑。
    ///
    /// 第一相（快照）：捕获周期开始时的不可变流水线状态。
    ///
    /// 第二相（组合逻辑）：HazardUnit 检测 Load-Use 冒险
    ///   （纯函数，无状态）。Alu 与 LSU 计算待完成结果。
    ///
    /// 第三相（时序逻辑）：按反向顺序提交流水线推进
    ///   （WB → MEM → EX → ID → IF）。反向顺序实现同周期
    ///   EX→ID 结果转发：ID 阶段读取刚更新的 EM/MW 锁存器。
    ///   HazardResult 控制停顿（冻结 IF，向 ID 插入气泡）。
    void step_cycle();

    /// 打印模拟统计汇总日志
    void print_statistics() const;

    /// 获取寄存器文件（供测试使用）
    const std::array<uint64_t, 32>& registers() const { return registers_; }

private:
    // ── 流水线锁存器 ──
    /// IF/ID：取指 → 译码
    std::optional<IfIdLatch> fd_latch_;
    /// ID/EX：译码 → 执行
    std::optional<IdExLatch> de_latch_;
    /// EX/MEM：执行 → 访存
    std::optional<ExMemLatch> em_latch_;
    /// MEM/WB：访存 → 写回
    std::optional<MemWbLatch> mw_latch_;

    /// Load-Store Unit（建模访存时序）
    Lsu lsu_;

    /// 体系结构寄存器文件（32 个整数寄存器，x0 硬连线为 0）
    std::array<uint64_t, 32> registers_;

    /// Trace 输入队列，按程序顺序存放待发射的指令
    std::deque<TraceRecord> trace_;

    /// 流水线停顿标志：为 true 时 IF 冻结、ID 插入气泡
    bool stall_ = false;

    /// 组合转发网络：解析源寄存器的值。
    ///
    /// 读取刚更新的 em_latch 和 mw_latch（在 EX/MEM 提交之后）
    /// 以建模 EX 到 ID 的同周期结果转发。
    ///
    /// 优先级（硬件转发多路选择器顺序）：
    ///   1. EX/MEM 的 alu_result（最近结果，Load 除外 — 数据尚未就绪）
    ///   2. MEM/WB 的 mem_result（Load）或 alu_result（非 Load）
    ///   3. 寄存器文件（默认）
    uint64_t forward_value(uint8_t rs);
};
