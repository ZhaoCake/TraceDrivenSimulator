#include "simulator/hazard.h"
#include "simulator/alu.h"

HazardResult HazardUnit::resolve(const PipelineSnapshot& snapshot) {
    HazardResult result;

    // ── 检测 Load-Use 冒险 ──
    // 条件：即将进入 EX 的指令（位于 ID/EX 锁存器中）是 Load，
    // 且 IF/ID 中的指令依赖其结果。
    //
    // 在两阶段设计中，快照在 EX/MEM 提交之前获取，
    // 因此本周期进入 EX 的指令仍在 de_latch 中。
    // Load 数据在 MEM/WB 之前不可用（需 2 个周期），
    // 因此依赖指令需停顿 1 周期。
    if (snapshot.de.has_value()) {
        const auto& de = snapshot.de.value();
        if (de.record.op_type == OpType::Load && de.record.rd != 0) {
            if (snapshot.fd.has_value()) {
                const auto& fd = snapshot.fd.value();
                if (fd.record.rs1 == de.record.rd || fd.record.rs2 == de.record.rd) {
                    result.stall_if = true;
                    result.stall_id = true;
                }
            }
        }
    }

    // ── 确定 ID 阶段的转发来源 ──
    // 仅当 ID 阶段有指令（fd_latch）时才计算转发
    if (snapshot.fd.has_value()) {
        const auto& fd = snapshot.fd.value();
        result.fwd_src1 = resolve_forward(fd.record.rs1, snapshot);
        result.fwd_src2 = resolve_forward(fd.record.rs2, snapshot);
    }

    return result;
}

ForwardSource HazardUnit::resolve_forward(uint8_t rs, const PipelineSnapshot& snap) {
    // x0 在 RISC-V 中硬连线为 0
    if (rs == 0) {
        return ForwardSource::reg_file(0);
    }

    // 优先级 1：ID/EX 预计算 — 同周期 EX→ID 转发。
    // de_latch 中的非 Load 指令将在本周期 EX 阶段计算完成，其结果可旁路到 ID 阶段。
    // Load 在 EX 阶段仅计算有效地址，数据尚未就绪，因此排除。
    if (snap.de.has_value()) {
        const auto& de = snap.de.value();
        if (de.record.rd == rs && de.record.op_type != OpType::Load) {
            uint64_t ex_result = Alu::compute(de.record, de.rs1_val, de.rs2_val);
            return ForwardSource::ex_result(ex_result);
        }
    }

    // 优先级 2：EX/MEM — 上一周期的结果。
    // 包括 Load：MEM 阶段先于 ID 执行（反向顺序），Load 数据在本周期进入 mw_latch，
    // ID 阶段可旁路拿到 mem_result。
    if (snap.em.has_value()) {
        const auto& em = snap.em.value();
        if (em.record.rd == rs) {
            if (em.record.op_type == OpType::Load) {
                return ForwardSource::mem_wb_mem(em.record.rd_val);
            } else {
                return ForwardSource::ex_mem_alu(em.alu_result);
            }
        }
    }

    // 优先级 3：MEM/WB — 前一个结果。
    if (snap.mw.has_value()) {
        const auto& mw = snap.mw.value();
        if (mw.record.rd == rs) {
            if (mw.record.op_type == OpType::Load) {
                return ForwardSource::mem_wb_mem(mw.mem_result);
            } else {
                return ForwardSource::mem_wb_alu(mw.alu_result);
            }
        }
    }

    // 优先级 4：寄存器文件 — 默认来源。
    return ForwardSource::reg_file(snap.registers[rs]);
}
