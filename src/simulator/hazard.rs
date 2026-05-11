use crate::trace::OpType;
use super::latches::PipelineSnapshot;

/// 指定转发操作数值的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardSource {
    /// 来自寄存器文件的值
    RegFile(u64),
    /// 来自 EX/MEM 锁存器的 ALU 结果（最近结果，Load 除外）
    ExMemAlu(u64),
    /// 来自 MEM/WB 锁存器的 ALU 结果
    MemWbAlu(u64),
    /// 来自 MEM/WB 锁存器的访存加载结果
    MemWbMem(u64),
}

impl ForwardSource {
    /// 从转发源中提取实际值。
    #[allow(dead_code)]
    pub fn value(&self) -> u64 {
        match *self {
            ForwardSource::RegFile(v) | ForwardSource::ExMemAlu(v)
            | ForwardSource::MemWbAlu(v) | ForwardSource::MemWbMem(v) => v,
        }
    }

    /// 若为转发命中（非寄存器文件）则返回 true。
    #[allow(dead_code)]
    pub fn is_forward(&self) -> bool {
        !matches!(self, ForwardSource::RegFile(_))
    }
}

/// 组合冒险解析的结果。
/// 所有字段均为时序提交阶段的控制信号。
#[derive(Debug, Clone)]
pub struct HazardResult {
    /// 冻结 IF/ID 锁存器（PC 不递增）
    pub stall_if: bool,
    /// 冻结 ID/EX 锁存器（插入气泡 / NOP）
    pub stall_id: bool,
    /// 冲刷 IF/ID 锁存器（如分支预测错误）
    #[allow(dead_code)]
    pub flush_if: bool,
    /// 冲刷 ID/EX 锁存器
    #[allow(dead_code)]
    pub flush_id: bool,
    /// ID 阶段指令的 rs1 转发源
    pub fwd_src1: ForwardSource,
    /// ID 阶段指令的 rs2 转发源
    pub fwd_src2: ForwardSource,
}

impl Default for HazardResult {
    fn default() -> Self {
        HazardResult {
            stall_if: false,
            stall_id: false,
            flush_if: false,
            flush_id: false,
            fwd_src1: ForwardSource::RegFile(0),
            fwd_src2: ForwardSource::RegFile(0),
        }
    }
}

/// 组合冒险检测与转发解析单元。
///
/// 纯函数 — 无状态、无寄存器、无时钟。
/// 接收流水线状态快照，产出控制信号供时序提交阶段使用。
pub struct HazardUnit;

impl HazardUnit {
    /// 解析所有冒险并确定转发来源。
    ///
    /// 输入：不可变流水线快照（周期开始时的锁存器状态）。
    /// 输出：控制信号（停顿/冲刷/转发），供提交阶段使用。
    pub fn resolve(snapshot: &PipelineSnapshot) -> HazardResult {
        let mut result = HazardResult::default();

        // ── 检测 Load-Use 冒险 ──
        // Condition: the instruction about to enter EX (in ID/EX latch)
        // is a Load, and the instruction in IF/ID depends on its result.
        //
        // In the two-phase design, the snapshot is taken BEFORE EX/MEM commit,
        // so the instruction entering EX this cycle is still in de_latch.
        // The Load data won't be available until MEM/WB (2 cycles later),
        // so the dependent instruction must stall for 1 cycle.
        if let Some(ref de) = snapshot.de {
            if de.record.op_type == OpType::Load && de.record.rd != 0 {
                if let Some(ref fd) = snapshot.fd {
                    if fd.record.rs1 == de.record.rd || fd.record.rs2 == de.record.rd {
                        result.stall_if = true;
                        result.stall_id = true;
                    }
                }
            }
        }

        // ── 确定 ID 阶段的转发来源 ──
        // Only compute forwarding if there's an instruction in ID (fd_latch).
        if let Some(ref fd) = snapshot.fd {
            result.fwd_src1 = Self::resolve_forward(fd.record.rs1, snapshot);
            result.fwd_src2 = Self::resolve_forward(fd.record.rs2, snapshot);
        }

        result
    }

    /// 解析单个寄存器操作数的转发来源。
    ///
    /// 优先级（硬件转发多路选择器顺序）：
    ///   1. x0 → 恒为 0（硬连线）
    ///   2. EX/MEM 的 alu_result（最近结果，Load 除外 — 数据未就绪）
    ///   3. MEM/WB 的 mem_result（Load）或 alu_result（其他）
    ///   4. 寄存器文件（默认/回退）
    fn resolve_forward(rs: u8, snap: &PipelineSnapshot) -> ForwardSource {
        // x0 is hardwired to 0 in RISC-V
        if rs == 0 {
            return ForwardSource::RegFile(0);
        }

        // Priority 1: EX/MEM — the most recent result.
        // IMPORTANT: Skip Load instructions — their data isn't ready until MEM completes.
        // The alu_result field of a Load in EX/MEM contains the effective address, not the data.
        if let Some(ref em) = snap.em {
            if em.record.rd == rs && em.record.op_type != OpType::Load {
                return ForwardSource::ExMemAlu(em.alu_result);
            }
        }

        // Priority 2: MEM/WB — the previous result.
        if let Some(ref mw) = snap.mw {
            if mw.record.rd == rs {
                return if mw.record.op_type == OpType::Load {
                    ForwardSource::MemWbMem(mw.mem_result)
                } else {
                    ForwardSource::MemWbAlu(mw.alu_result)
                };
            }
        }

        // Priority 3: Register file — default source.
        ForwardSource::RegFile(snap.registers[rs as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{OpType, TraceRecord};
    use super::super::latches::*;

    fn make_record(pc: u64, rs1: u8, rs2: u8, rd: u8, op_type: OpType, rd_val: u64) -> TraceRecord {
        TraceRecord { pc, inst: 0, rs1, rs2, rd, op_type, mem_addr: 0, rd_val }
    }

    #[test]
    fn test_load_use_hazard_detected() {
        let load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
        let alu_rec = make_record(0x104, 1, 0, 2, OpType::IntAlu, 199);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(alu_rec)),  // FD: ALU 依赖 x1
            Some(IdExLatch::new(load_rec, 0, 0)), // DE: Load 写入 x1
            None,  // EM: 空
            None,
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert!(result.stall_if, "Load-Use 冒险时应停顿 IF");
        assert!(result.stall_id, "Load-Use 冒险时应停顿 ID（插入气泡）");
    }

    #[test]
    fn test_load_use_no_hazard_when_rd_is_x0() {
        // Load 写入 x0（结果丢弃）— 无冒险
        let load_rec = make_record(0x100, 4, 0, 0, OpType::Load, 99);
        let alu_rec = make_record(0x104, 0, 0, 1, OpType::IntAlu, 42);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(alu_rec)),
            Some(IdExLatch::new(load_rec, 0, 0)), // DE: Load 写入 x0
            None,
            None,
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert!(!result.stall_if, "Load 写入 x0 时不应停顿");
        assert!(!result.stall_id, "Load 写入 x0 时不应插入气泡");
    }

    #[test]
    fn test_forward_x0_always_zero() {
        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(make_record(0x100, 0, 0, 1, OpType::IntAlu, 42))),
            None, None, None,
            [99; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert_eq!(result.fwd_src1.value(), 0);
        // x0 不算转发（硬连线为 0）
        assert!(!result.fwd_src1.is_forward());
    }

    #[test]
    fn test_forward_from_ex_mem_alu() {
        let producer = make_record(0x100, 4, 5, 1, OpType::IntAlu, 42);
        let consumer = make_record(0x104, 1, 0, 2, OpType::IntAlu, 84);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)), // FD: 需要 x1
            None,
            Some(ExMemLatch::new(producer, 42)), // EM: 刚产出 x1=42
            None,
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        // consumer 的 rs1 (=1) 应从 EX/MEM 转发
        assert_eq!(result.fwd_src1, ForwardSource::ExMemAlu(42));
        assert!(result.fwd_src1.is_forward());
        // consumer 的 rs2 (=0) 是 x0
        assert!(!result.fwd_src2.is_forward());
    }

    #[test]
    fn test_load_not_forwarded_from_ex_mem() {
        // 当 Load 即将进入 EX（位于 de_latch，两阶段快照
        // 在 EX/MEM 提交之前获取）且 consumer 即将进入 ID
        // （位于 fd_latch）时，Load 数据在 MEM/WB 之前不可用。
        // 无法转发，且必须进行 Load-Use 停顿。
        let load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
        let consumer = make_record(0x104, 1, 0, 2, OpType::IntAlu, 199);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)),        // FD: consumer 依赖 x1
            Some(IdExLatch::new(load_rec, 0, 0)),  // DE: Load 写入 x1，即将进入 EX
            None,                                   // EM: 空
            None,
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        // Load 数据未就绪 → 不应从任何地方转发
        assert!(!result.fwd_src1.is_forward(), "Load 数据未就绪，无法转发");
        // 检测到 Load-Use 冒险 → 需要停顿
        assert!(result.stall_if);
        assert!(result.stall_id);
    }

    #[test]
    fn test_forward_from_mem_wb_load() {
        // MEM/WB 中有 Load：mem_result 存有加载数据。
        let load_rec = make_record(0x100, 4, 0, 1, OpType::Load, 99);
        let consumer = make_record(0x108, 1, 0, 2, OpType::IntAlu, 199);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)),
            None,
            None,
            Some(MemWbLatch::new(load_rec, 0x8000, 99)), // mem_result=99
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert_eq!(result.fwd_src1, ForwardSource::MemWbMem(99));
        assert!(result.fwd_src1.is_forward());
    }

    #[test]
    fn test_forward_from_mem_wb_alu() {
        let producer = make_record(0x100, 4, 5, 1, OpType::IntAlu, 42);
        let consumer = make_record(0x108, 1, 0, 2, OpType::IntAlu, 84);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)),
            None,
            None,
            Some(MemWbLatch::new(producer, 42, 0)),
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert_eq!(result.fwd_src1, ForwardSource::MemWbAlu(42));
        assert!(result.fwd_src1.is_forward());
    }

    #[test]
    fn test_forward_priority_ex_mem_over_mem_wb() {
        // 同一寄存器在 EX/MEM 和 MEM/WB 中均有写入。
        // EX/MEM 更新 — 应具有更高优先级。
        let old = make_record(0x100, 0, 0, 1, OpType::IntAlu, 100); // x1 = 100
        let new = make_record(0x104, 0, 0, 1, OpType::IntAlu, 200); // x1 = 200
        let consumer = make_record(0x108, 1, 0, 5, OpType::IntAlu, 300);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)),
            None,
            Some(ExMemLatch::new(new, 200)),      // 较新的 x1=200
            Some(MemWbLatch::new(old, 100, 0)),    // 较旧的 x1=100
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        // 应取 EX/MEM (200) 而非 MEM/WB (100)
        assert_eq!(result.fwd_src1, ForwardSource::ExMemAlu(200));
    }

    #[test]
    fn test_no_forward_when_rs_not_matching() {
        let producer = make_record(0x100, 0, 0, 5, OpType::IntAlu, 42);
        let consumer = make_record(0x104, 1, 2, 3, OpType::IntAlu, 99);

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)), // 需要 x1、x2
            None,
            Some(ExMemLatch::new(producer, 42)), // 写入 x5（不匹配）
            None,
            [0; 32],
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert!(!result.fwd_src1.is_forward(), "x1 应来自寄存器文件");
        assert!(!result.fwd_src2.is_forward(), "x2 应来自寄存器文件");
    }

    #[test]
    fn test_fallback_to_register_file() {
        let consumer = make_record(0x100, 3, 4, 1, OpType::IntAlu, 42);
        let mut regs = [0u64; 32];
        regs[3] = 777;
        regs[4] = 888;

        let snap = PipelineSnapshot::new(
            Some(IfIdLatch::new(consumer)),
            None, None, None,
            regs,
            0,
        );

        let result = HazardUnit::resolve(&snap);
        assert_eq!(result.fwd_src1.value(), 777);
        assert_eq!(result.fwd_src2.value(), 888);
        assert!(!result.fwd_src1.is_forward());
        assert!(!result.fwd_src2.is_forward());
    }
}
