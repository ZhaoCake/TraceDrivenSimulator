#pragma once

#include <cstdint>
#include "trace.h"

/// 加载存储单元（LSU）— 建模访存时序。
///
/// Trace 驱动模拟器中，LSU 不访问真实内存（Trace 提供加载值）。
/// 它仅建模访存操作后数据何时可用。
///
/// 经典五级流水线中：
///   - Load 延迟 = 1 周期（假设 L1 命中）
///   - 完全流水化（无结构冒险）
///
/// 未来扩展：
///   - 缓存缺失惩罚（延迟 > 1，缺失期间非流水化）
///   - Store 缓冲 / 写合并
///   - 乱序执行的加载存储队列（LSQ）
///   - Store-to-Load 转发
class Lsu {
public:
    /// 访存延迟（周期数）
    uint32_t latency;
    /// LSU 是否每周期可接收新请求
    bool pipelined;

    /// 创建新的 LSU。
    /// 默认：1 周期延迟，完全流水化（经典五级流水线）。
    Lsu();

    /// 检查 LSU 本周期能否接收新访存请求。
    bool is_ready(uint64_t current_cycle) const;

    /// 为访存操作预留 LSU。
    /// 返回数据可用的周期号。
    uint64_t reserve(uint64_t current_cycle);

    /// 执行访存操作（Trace 驱动：返回 Trace 提供的数据）。
    ///
    /// Load 指令：返回 Trace 中的加载值。
    /// Store 指令：返回 0（无需写回值）。
    /// 非访存指令：返回 0（不应调用）。
    uint64_t execute(const TraceRecord& record) const;

private:
    /// LSU 变为空闲的周期号（非流水化时使用）
    uint64_t busy_until_;
};
