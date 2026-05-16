#include <gtest/gtest.h>
#include <vector>
#include "simulator/simulator.h"

// ── TraceRecord 构造辅助函数 ──

/// 构造一条 IntAlu 指令的 TraceRecord
static TraceRecord make_alu(uint64_t pc, uint8_t rs1, uint8_t rs2, uint8_t rd, uint64_t rd_val) {
    return TraceRecord{pc, 0, rs1, rs2, rd, OpType::IntAlu, 0, rd_val};
}

/// 构造一条 Load 指令的 TraceRecord
static TraceRecord make_load(uint64_t pc, uint8_t rs1, uint8_t rd, uint64_t rd_val) {
    return TraceRecord{pc, 0, rs1, 0, rd, OpType::Load, 0x80000000, rd_val};
}

/// 构造一条 Store 指令的 TraceRecord
static TraceRecord make_store(uint64_t pc, uint8_t rs1, uint8_t rs2, uint64_t mem_addr) {
    return TraceRecord{pc, 0, rs1, rs2, 0, OpType::Store, mem_addr, 0};
}

/// 构造一条 Jump 指令的 TraceRecord
static TraceRecord make_jump(uint64_t pc, uint8_t rd) {
    return TraceRecord{pc, 0, 0, 0, rd, OpType::Jump, 0, 0};
}

/// 构造一条 Branch 指令的 TraceRecord
static TraceRecord make_branch(uint64_t pc, uint8_t rs1, uint8_t rs2) {
    return TraceRecord{pc, 0, rs1, rs2, 0, OpType::Branch, 0, 0};
}

// ── 辅助执行函数 ──

/// 加载 trace 并运行至流水线排空
static void run_sim(Simulator& sim, const std::vector<TraceRecord>& records) {
    sim.load_trace(records);
    while (!sim.is_done()) {
        sim.step_cycle();
    }
}

// ── 15 个集成测试用例 ──

/// 验证基本流水线功能：指令正确流过各阶段并写回
TEST(SimulatorTest, BasicPipelineFlow) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 1, 10),
        make_alu(0x104, 0, 0, 2, 20),
        make_alu(0x108, 0, 0, 3, 30),
    });

    // 3 条指令在五级流水线中：5 周期填充 + 2 周期排空 = 7 周期
    EXPECT_EQ(sim.cycle, 7u);
    EXPECT_EQ(sim.inst_count, 3u);
    EXPECT_EQ(sim.registers()[1], 10u);
    EXPECT_EQ(sim.registers()[2], 20u);
    EXPECT_EQ(sim.registers()[3], 30u);
}

/// 验证 ALU → ALU 转发：后续指令直接从 EX/MEM 取得操作数
TEST(SimulatorTest, AluForwarding) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 4, 5, 1, 42),  // x1 = 42
        make_alu(0x104, 1, 0, 2, 84),  // x2 = x1 + 0（依赖 x1）
    });

    // 转发应命中至少 1 次（第二条指令的 rs1 从 EX/MEM 获取）
    EXPECT_GE(sim.forward_count, 1u);
    EXPECT_EQ(sim.registers()[1], 42u);
    EXPECT_EQ(sim.registers()[2], 84u);
    // 无 Load-Use 停顿
    EXPECT_EQ(sim.stall_count, 0u);
}

/// 验证 Load-Use 冒险检测与停顿：Load 后紧跟使用其结果的指令会停顿一周期
TEST(SimulatorTest, LoadUseStall) {
    Simulator sim;
    run_sim(sim, {
        make_load(0x100, 4, 1, 99),    // lw x1, 0(x4) → x1 = 99
        make_alu(0x104, 1, 0, 2, 199), // add x2, x1, x0（依赖 Load 结果）
    });

    // 应发生 1 次 Load-Use 停顿
    EXPECT_EQ(sim.stall_count, 1u);
    EXPECT_EQ(sim.registers()[1], 99u);
    EXPECT_EQ(sim.registers()[2], 199u);
}

/// 验证连续无依赖指令的 IPC 接近 1.0
TEST(SimulatorTest, IndependentInstructionsIPC) {
    Simulator sim;
    uint64_t n = 100;
    std::vector<TraceRecord> records;
    for (uint64_t i = 0; i < n; i++) {
        records.push_back(make_alu(i * 4, 0, 0, static_cast<uint8_t>(i % 31 + 1), i));
    }
    run_sim(sim, records);

    uint64_t expected_cycles = 4 + n;
    EXPECT_EQ(sim.cycle, expected_cycles);
    EXPECT_EQ(sim.inst_count, n);
    double ipc = static_cast<double>(sim.inst_count) / sim.cycle;
    EXPECT_GT(ipc, 0.9);
}

/// 验证 Store 指令正确流经流水线且不写回寄存器
TEST(SimulatorTest, StorePipeline) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 5, 0xDEAD),
        make_store(0x104, 5, 0, 0x1000),
    });

    EXPECT_EQ(sim.inst_count, 2u);
    // Store 使用了 ALU 的结果 x5，应有一次转发
    EXPECT_GE(sim.forward_count, 1u);
}

/// 验证 ALU → ALU → ALU 转发链：连续三条依赖指令
TEST(SimulatorTest, AluForwardingChain) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 1, 10),   // x1 = 10
        make_alu(0x104, 1, 0, 2, 20),   // x2 = x1 + 10（依赖 x1）
        make_alu(0x108, 2, 0, 3, 30),   // x3 = x2 + 10（依赖 x2）
    });

    EXPECT_EQ(sim.registers()[1], 10u);
    EXPECT_EQ(sim.registers()[2], 20u);
    EXPECT_EQ(sim.registers()[3], 30u);
    // 至少 2 次转发（inst1←inst0, inst2←inst1）
    EXPECT_GE(sim.forward_count, 2u);
    EXPECT_EQ(sim.stall_count, 0u);
}

/// 验证 Load 结果隔一条指令后使用（无需停顿，MEM/WB 转发即可）
TEST(SimulatorTest, LoadResultForwardedFromMWB) {
    Simulator sim;
    run_sim(sim, {
        make_load(0x100, 4, 1, 99),      // lw x1, 0(x4) → x1 = 99
        make_alu(0x104, 0, 0, 5, 50),    // 独立 ALU（填充一个槽位）
        make_alu(0x108, 1, 0, 2, 199),   // add x2, x1, x0（隔一条使用 Load 结果）
    });

    // 无停顿：消费者与 Load 间隔一条指令，Load 已在 MEM/WB 就绪
    EXPECT_EQ(sim.stall_count, 0u);
    EXPECT_EQ(sim.registers()[1], 99u);
    EXPECT_EQ(sim.registers()[2], 199u);
    // 应有一次 MW→ID 转发
    EXPECT_GE(sim.forward_count, 1u);
}

/// 验证连续两条 Load 后紧跟同时依赖两者的 ALU 指令
TEST(SimulatorTest, BackToBackLoadUse) {
    Simulator sim;
    run_sim(sim, {
        make_load(0x100, 4, 1, 10),      // lw x1, 0(x4)
        make_load(0x104, 5, 2, 20),      // lw x2, 0(x5)
        make_alu(0x108, 1, 2, 3, 30),    // add x3, x1, x2（依赖两个 Load）
    });

    EXPECT_EQ(sim.registers()[1], 10u);
    EXPECT_EQ(sim.registers()[2], 20u);
    EXPECT_EQ(sim.registers()[3], 30u);
    // 仅第二次 Load 会触发停顿（第一条 Load 已在 MEM/WB 转发就绪）
    EXPECT_EQ(sim.stall_count, 1u);
}

/// 验证 Jump 指令：JAL 的返回地址 (pc+4) 正确写回 rd
TEST(SimulatorTest, JumpLinkWriteback) {
    Simulator sim;
    uint64_t jump_pc = 0x100;
    run_sim(sim, {
        make_jump(jump_pc, 1), // jal x1, target → x1 = pc + 4
    });

    EXPECT_EQ(sim.registers()[1], jump_pc + 4);
    EXPECT_EQ(sim.inst_count, 1u);
}

/// 验证 Branch 指令正常流经流水线（不写回寄存器）
TEST(SimulatorTest, BranchPipelineFlow) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 5, 5),
        make_branch(0x104, 5, 0), // beq x5, x0
    });

    EXPECT_EQ(sim.inst_count, 2u);
    // Branch 用到了 x5，应从转发获取
    EXPECT_GE(sim.forward_count, 1u);
}

/// 验证 Store 指令的 rs2 值能从之前指令正确转发
TEST(SimulatorTest, StoreRs2Forwarding) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 5, 0xABCD),   // x5 = 0xABCD
        make_alu(0x104, 5, 0, 6, 0x5678),   // x6 = x5 + ... (依赖 x5)
        make_store(0x108, 10, 6, 0x8000),    // sw x6, 0(x10)（依赖 x6）
    });

    EXPECT_EQ(sim.inst_count, 3u);
    // x5→x6 一次转发，x6→store 一次转发
    EXPECT_GE(sim.forward_count, 2u);
    EXPECT_EQ(sim.stall_count, 0u);
    EXPECT_EQ(sim.registers()[5], 0xABCDu);
    EXPECT_EQ(sim.registers()[6], 0x5678u);
}

/// 验证 x0 寄存器始终为 0，即使有指令试图写 x0
TEST(SimulatorTest, X0AlwaysZero) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 0, 999), // 试图写 x0
        make_alu(0x104, 0, 0, 0, 888), // 再次试图写 x0
    });

    // x0 始终为 0
    EXPECT_EQ(sim.registers()[0], 0u);
    // 但指令仍应被计数
    EXPECT_EQ(sim.inst_count, 2u);
}

/// 验证转发优先级：EX/MEM 优先于 MEM/WB（最近的结果优先）
TEST(SimulatorTest, ForwardingPriorityEmOverMw) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 3, 100),  // x3 = 100
        make_alu(0x104, 0, 0, 3, 200),  // x3 = 200（覆盖 x3）
        make_alu(0x108, 3, 0, 5, 300),  // x5 = x3 + ...（应取 EX/MEM 中的 200）
    });

    // x5 应使用最近写入 x3 的值 200
    EXPECT_EQ(sim.registers()[3], 200u);
    EXPECT_EQ(sim.registers()[5], 300u);
    // 第三次指令应从 EX/MEM 转发（而非 MEM/WB）
    EXPECT_GE(sim.forward_count, 1u);
}

/// 验证空 Trace 时 is_done 立即为 true
TEST(SimulatorTest, EmptyTrace) {
    Simulator sim;
    EXPECT_TRUE(sim.is_done());
    EXPECT_EQ(sim.cycle, 0u);
    EXPECT_EQ(sim.inst_count, 0u);
}

/// 验证混合指令类型的操作统计分布
TEST(SimulatorTest, OpStatsDistribution) {
    Simulator sim;
    run_sim(sim, {
        make_alu(0x100, 0, 0, 1, 10),
        make_load(0x104, 1, 2, 20),
        make_alu(0x108, 2, 0, 3, 30),
        make_store(0x10C, 3, 0, 0x8000),
        make_branch(0x110, 1, 2),
        make_jump(0x114, 5),
    });

    EXPECT_EQ(sim.inst_count, 6u);
    // Count is key-based lookup, default = 0
    auto count_or_zero = [&](OpType op) -> uint64_t {
        auto it = sim.op_stats.find(op);
        return (it != sim.op_stats.end()) ? it->second : 0;
    };
    EXPECT_EQ(count_or_zero(OpType::IntAlu), 2u);
    EXPECT_EQ(count_or_zero(OpType::Load), 1u);
    EXPECT_EQ(count_or_zero(OpType::Store), 1u);
    EXPECT_EQ(count_or_zero(OpType::Branch), 1u);
    EXPECT_EQ(count_or_zero(OpType::Jump), 1u);
}
