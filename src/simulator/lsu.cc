#include "simulator/lsu.h"

Lsu::Lsu() : latency(1), pipelined(true), busy_until_(0) {}

bool Lsu::is_ready(uint64_t current_cycle) const {
    // 流水化：始终可接收；非流水化：空闲时才能接收
    return pipelined || current_cycle >= busy_until_;
}

uint64_t Lsu::reserve(uint64_t current_cycle) {
    uint64_t ready_cycle = current_cycle + latency;
    // 非流水化 LSU：标记忙直到 ready_cycle
    if (!pipelined) {
        busy_until_ = ready_cycle;
    }
    return ready_cycle;
}

uint64_t Lsu::execute(const TraceRecord& record) const {
    // Trace 驱动：Load 返回 Trace 中的 rd_val，其他返回 0
    if (record.op_type == OpType::Load) {
        return record.rd_val;
    }
    return 0;
}
