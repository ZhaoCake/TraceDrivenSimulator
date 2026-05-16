#include <gtest/gtest.h>
#include "simulator/alu.h"

// 构造测试用的 TraceRecord
static TraceRecord make_record(uint64_t pc, OpType op_type, uint64_t mem_addr, uint64_t rd_val) {
    return TraceRecord{pc, 0, 0, 0, 1, op_type, mem_addr, rd_val};
}

/// 测试 IntAlu 指令使用 rd_val 作为 ALU 结果
TEST(AluTest, IntAluUsesRdVal) {
    auto rec = make_record(0x100, OpType::IntAlu, 0, 42);
    EXPECT_EQ(Alu::compute(rec, 10, 32), 42);
}

/// 测试 Load 指令使用 mem_addr 作为 ALU 结果（有效地址）
TEST(AluTest, LoadUsesMemAddr) {
    auto rec = make_record(0x100, OpType::Load, 0x80000000, 99);
    EXPECT_EQ(Alu::compute(rec, 0, 0), 0x80000000);
}

/// 测试 Store 指令使用 mem_addr 作为 ALU 结果（有效地址）
TEST(AluTest, StoreUsesMemAddr) {
    auto rec = make_record(0x100, OpType::Store, 0x1000, 0);
    EXPECT_EQ(Alu::compute(rec, 0, 0), 0x1000);
}

/// 测试 Jump 指令使用 pc + 4 作为 ALU 结果（返回地址）
TEST(AluTest, JumpUsesPcPlus4) {
    auto rec = make_record(0x200, OpType::Jump, 0, 0);
    EXPECT_EQ(Alu::compute(rec, 0, 0), 0x204);
}

/// 测试 Branch 指令使用 rd_val 作为 ALU 结果
TEST(AluTest, BranchUsesRdVal) {
    auto rec = make_record(0x100, OpType::Branch, 0, 1);
    EXPECT_EQ(Alu::compute(rec, 0, 0), 1);
}
