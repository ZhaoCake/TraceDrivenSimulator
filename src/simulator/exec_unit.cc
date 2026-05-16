#include "simulator/exec_unit.h"

/// 创建新的执行单元。
/// 所有字段由参数初始化，busy_until_ 初始为 0（空闲）。
ExecUnit::ExecUnit(std::string name, uint32_t latency, bool pipelined)
    : name(std::move(name))
    , latency(latency)
    , pipelined(pipelined)
    , busy_until_(0)
{
}

/// 创建默认整数 ALU 单元。
/// 名称 "IntAlu"，延迟 1 周期，完全流水化。
ExecUnit ExecUnit::default_int_alu() {
    return ExecUnit("IntAlu", 1, true);
}

/// 创建默认加载/存储单元。
/// 名称 "LSU"，延迟 1 周期，完全流水化。
ExecUnit ExecUnit::default_lsu() {
    return ExecUnit("LSU", 1, true);
}

/// 创建默认分支单元。
/// 名称 "Branch"，延迟 1 周期，完全流水化。
ExecUnit ExecUnit::default_branch() {
    return ExecUnit("Branch", 1, true);
}

/// 检查本周期能否接收新指令。
/// 流水化单元：始终返回 true（无结构冒险）。
/// 非流水化单元：当前周期 >= busy_until_ 时返回 true（单元空闲）。
bool ExecUnit::is_ready(uint64_t current_cycle) const {
    return pipelined || current_cycle >= busy_until_;
}

/// 为从 current_cycle 开始的指令预留此单元。
/// 返回结果可用的周期号 = current_cycle + latency。
/// 对于非流水化单元，同时设置 busy_until_ 为 ready_cycle，
/// 阻止后续指令在同一周期发射。
uint64_t ExecUnit::reserve(uint64_t current_cycle) {
    uint64_t ready_cycle = current_cycle + latency;
    if (!pipelined) {
        busy_until_ = ready_cycle;
    }
    return ready_cycle;
}

/// 释放单元，将 busy_until_ 重置为 0。
/// 非流水化单元在结果被消费后调用，解除结构冒险。
void ExecUnit::release() {
    busy_until_ = 0;
}
