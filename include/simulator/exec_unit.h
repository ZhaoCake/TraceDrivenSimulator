#pragma once

#include <string>
#include <cstdint>

/// 建模执行单元的时序行为。
///
/// Trace 驱动模拟器中，执行单元不执行真实计算（Trace 提供结果）。
/// 它仅建模结果何时可用——即执行延迟与结构冒险检测。
///
/// 经典五级流水线中，所有单元延迟=1 且完全流水化（每周期可接收新指令）。
///
/// 未来扩展：
///   - 乘法器：延迟=3，流水化=true
///   - 除法器：延迟=20，非流水化（结构冒险）
class ExecUnit {
public:
    /// 人类可读名称，用于调试/统计
    std::string name;
    /// 从发射到结果可用的周期数
    uint32_t latency;
    /// 此单元是否每周期可接收新指令
    bool pipelined;

    /// 创建新的执行单元。
    /// \param name       人类可读名称
    /// \param latency    执行延迟（周期数）
    /// \param pipelined  是否完全流水化
    ExecUnit(std::string name, uint32_t latency, bool pipelined);

    /// 创建默认整数 ALU 单元（1 周期，完全流水化）。
    static ExecUnit default_int_alu();

    /// 创建默认加载/存储单元（1 周期，完全流水化）。
    static ExecUnit default_lsu();

    /// 创建默认分支单元（1 周期，完全流水化）。
    static ExecUnit default_branch();

    /// 检查单元本周期能否接收新指令。
    /// 流水化单元始终返回 true（无结构冒险）。
    /// 非流水化单元仅在空闲时返回 true。
    bool is_ready(uint64_t current_cycle) const;

    /// 为从指定周期开始的指令预留此单元。
    /// 返回结果可用的周期号。
    /// 非流水化单元：同时更新 busy_until_ 以标记占用。
    uint64_t reserve(uint64_t current_cycle);

    /// 释放单元（非流水化单元在结果被消费后调用）。
    void release();

private:
    /// 单元变为空闲的周期号（非流水化单元时使用）
    uint64_t busy_until_;
};
