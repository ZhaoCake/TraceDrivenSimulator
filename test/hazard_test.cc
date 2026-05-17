#include <gtest/gtest.h>
#include <array>
#include "simulator/hazard.h"

// 构造测试用的 TraceRecord
static TraceRecord make_record(
    uint64_t pc, uint8_t rs1, uint8_t rs2, uint8_t rd,
    OpType op_type, uint64_t rd_val
) {
    return TraceRecord{pc, 0, rs1, rs2, rd, op_type, 0, rd_val};
}

/// 测试 Load-Use 冒险检测：Load 后紧跟使用其结果的 ALU 指令应触发停顿
TEST(HazardTest, LoadUseHazardDetected) {
    auto load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
    auto alu_rec = make_record(0x104, 1, 0, 2, OpType::IntAlu, 199);

    PipelineSnapshot snap(
        IfIdLatch(alu_rec),                         // FD: ALU 依赖 x1
        IdExLatch(load_rec, 0, 0),                  // DE: Load 写入 x1
        std::nullopt,                                // EM: 空
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_TRUE(result.stall_if) << "Load-Use 冒险时应停顿 IF";
    EXPECT_TRUE(result.stall_id) << "Load-Use 冒险时应停顿 ID（插入气泡）";
}

/// 测试 Load 写入 x0 时不触发冒险（x0 硬连线，结果丢弃）
TEST(HazardTest, LoadUseNoHazardWhenRdIsX0) {
    auto load_rec = make_record(0x100, 4, 0, 0, OpType::Load, 99);
    auto alu_rec = make_record(0x104, 0, 0, 1, OpType::IntAlu, 42);

    PipelineSnapshot snap(
        IfIdLatch(alu_rec),
        IdExLatch(load_rec, 0, 0),                  // DE: Load 写入 x0
        std::nullopt,
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_FALSE(result.stall_if) << "Load 写入 x0 时不应停顿";
    EXPECT_FALSE(result.stall_id) << "Load 写入 x0 时不应插入气泡";
}

/// 测试 x0 寄存器始终转发为 0（不算转发命中）
TEST(HazardTest, ForwardX0AlwaysZero) {
    std::array<uint64_t, 32> regs;
    regs.fill(99);

    PipelineSnapshot snap(
        IfIdLatch(make_record(0x100, 0, 0, 1, OpType::IntAlu, 42)),
        std::nullopt, std::nullopt, std::nullopt,
        regs,
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_EQ(result.fwd_src1.get_value(), 0u);
    // x0 不算转发（硬连线为 0）
    EXPECT_FALSE(result.fwd_src1.is_forward());
}

/// 测试从 EX/MEM ALU 转发（最近的非 Load 结果）
TEST(HazardTest, ForwardFromExMemAlu) {
    auto producer = make_record(0x100, 4, 5, 1, OpType::IntAlu, 42);
    auto consumer = make_record(0x104, 1, 0, 2, OpType::IntAlu, 84);

    PipelineSnapshot snap(
        IfIdLatch(consumer),                         // FD: 需要 x1
        std::nullopt,
        ExMemLatch(producer, 42),                    // EM: 刚产出 x1=42
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    // consumer 的 rs1 (=1) 应从 EX/MEM 转发
    EXPECT_EQ(result.fwd_src1, ForwardSource::ex_mem_alu(42));
    EXPECT_TRUE(result.fwd_src1.is_forward());
    // consumer 的 rs2 (=0) 是 x0
    EXPECT_FALSE(result.fwd_src2.is_forward());
}

/// 测试 Load 在 EX/MEM 时不应被转发（数据在 MEM/WB 之前未就绪）
TEST(HazardTest, LoadNotForwardedFromExMem) {
    auto load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
    auto consumer = make_record(0x104, 1, 0, 2, OpType::IntAlu, 199);

    PipelineSnapshot snap(
        IfIdLatch(consumer),                         // FD: consumer 依赖 x1
        IdExLatch(load_rec, 0, 0),                   // DE: Load 写入 x1，即将进入 EX
        std::nullopt,                                 // EM: 空
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    // Load 数据未就绪 → 不应从任何地方转发
    EXPECT_FALSE(result.fwd_src1.is_forward()) << "Load 数据未就绪，无法转发";
    // 检测到 Load-Use 冒险 → 需要停顿
    EXPECT_TRUE(result.stall_if);
    EXPECT_TRUE(result.stall_id);
}

/// 测试从 MEM/WB Load 转发（mem_result 存有加载数据）
TEST(HazardTest, ForwardFromMemWbLoad) {
    auto load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
    auto consumer = make_record(0x108, 1, 0, 2, OpType::IntAlu, 199);

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        std::nullopt,
        std::nullopt,
        MemWbLatch(load_rec, 0x8000, 99),            // MW: mem_result=99
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_EQ(result.fwd_src1, ForwardSource::mem_wb_mem(99));
    EXPECT_TRUE(result.fwd_src1.is_forward());
}

/// 测试从 MEM/WB ALU 转发（非 Load 指令的结果）
TEST(HazardTest, ForwardFromMemWbAlu) {
    auto producer = make_record(0x100, 4, 5, 1, OpType::IntAlu, 42);
    auto consumer = make_record(0x108, 1, 0, 2, OpType::IntAlu, 84);

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        std::nullopt,
        std::nullopt,
        MemWbLatch(producer, 42, 0),
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_EQ(result.fwd_src1, ForwardSource::mem_wb_alu(42));
    EXPECT_TRUE(result.fwd_src1.is_forward());
}

/// 测试转发优先级：EX/MEM 优先于 MEM/WB（同一寄存器，取最近结果）
TEST(HazardTest, ForwardPriorityExMemOverMemWb) {
    auto old_rec = make_record(0x100, 0, 0, 1, OpType::IntAlu, 100);  // x1 = 100
    auto new_rec = make_record(0x104, 0, 0, 1, OpType::IntAlu, 200);  // x1 = 200
    auto consumer = make_record(0x108, 1, 0, 5, OpType::IntAlu, 300);

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        std::nullopt,
        ExMemLatch(new_rec, 200),                    // 较新的 x1=200
        MemWbLatch(old_rec, 100, 0),                  // 较旧的 x1=100
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    // 应取 EX/MEM (200) 而非 MEM/WB (100)
    EXPECT_EQ(result.fwd_src1, ForwardSource::ex_mem_alu(200));
}

/// 测试 rs 不匹配时无转发，回退到寄存器文件
TEST(HazardTest, NoForwardWhenRsNotMatching) {
    auto producer = make_record(0x100, 0, 0, 5, OpType::IntAlu, 42);  // 写入 x5
    auto consumer = make_record(0x104, 1, 2, 3, OpType::IntAlu, 99);  // 需要 x1、x2

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        std::nullopt,
        ExMemLatch(producer, 42),                    // 写入 x5（不匹配）
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_FALSE(result.fwd_src1.is_forward()) << "x1 应来自寄存器文件";
    EXPECT_FALSE(result.fwd_src2.is_forward()) << "x2 应来自寄存器文件";
}

/// 测试从同周期 EX 阶段预计算转发（de_latch 中非 Load 指令的 ALU 结果）
TEST(HazardTest, ForwardFromExResult) {
    auto producer = make_record(0x100, 0, 0, 1, OpType::IntAlu, 42);  // x1 = 42
    auto consumer = make_record(0x104, 1, 0, 2, OpType::IntAlu, 84);  // 需要 x1

    PipelineSnapshot snap(
        IfIdLatch(consumer),                         // FD: 依赖 x1
        IdExLatch(producer, 0, 0),                   // DE: 即将生产 x1=42
        std::nullopt,                                 // EM: 空
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_EQ(result.fwd_src1, ForwardSource::ex_result(42));
    EXPECT_TRUE(result.fwd_src1.is_forward());
}

/// 测试转发优先级：同周期 EX 结果优先于 EX/MEM（更近的程序顺序）
TEST(HazardTest, ForwardPriorityExResultOverExMem) {
    auto newer = make_record(0x104, 0, 0, 1, OpType::IntAlu, 200);  // x1 = 200 (de_latch)
    auto older = make_record(0x100, 0, 0, 1, OpType::IntAlu, 100);  // x1 = 100 (em_latch)
    auto consumer = make_record(0x108, 1, 0, 5, OpType::IntAlu, 300);

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        IdExLatch(newer, 0, 0),                      // DE: 较新的 x1=200
        ExMemLatch(older, 100),                       // EM: 较旧的 x1=100
        std::nullopt,
        std::array<uint64_t, 32>{},
        0
    );

    auto result = HazardUnit::resolve(snap);
    // 应取 ExResult (200) 而非 ExMemAlu (100)
    EXPECT_EQ(result.fwd_src1, ForwardSource::ex_result(200));
}

/// 测试无转发命中时回退到寄存器文件中的值
TEST(HazardTest, FallbackToRegisterFile) {
    auto consumer = make_record(0x100, 3, 4, 1, OpType::IntAlu, 42);
    std::array<uint64_t, 32> regs{};
    regs[3] = 777;
    regs[4] = 888;

    PipelineSnapshot snap(
        IfIdLatch(consumer),
        std::nullopt, std::nullopt, std::nullopt,
        regs,
        0
    );

    auto result = HazardUnit::resolve(snap);
    EXPECT_EQ(result.fwd_src1.get_value(), 777u);
    EXPECT_EQ(result.fwd_src2.get_value(), 888u);
    EXPECT_FALSE(result.fwd_src1.is_forward());
    EXPECT_FALSE(result.fwd_src2.is_forward());
}
