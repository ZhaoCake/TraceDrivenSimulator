use crate::trace::{OpType, TraceRecord};

/// 组合 ALU（算术逻辑单元）。
///
/// Trace 驱动模拟器中，ALU 不执行真实算术运算
/// （Trace 提供预计算结果）。它仅从 Trace 记录中路由正确的值
/// 到流水线锁存器。
///
/// 纯组合函数 — 无状态、无时钟。
pub struct Alu;

impl Alu {
    /// 计算指令的 ALU 结果。
    ///
    /// 对于 Trace 驱动的模拟器，Trace 提供了真实值：
    ///   - 加载/存储：`mem_addr` 是实际 CPU 计算的有效地址
    ///   - 跳转：`pc + 4` 是返回地址（JAL 链接）
    ///   - 其他所有情况：`rd_val` 是 Trace 中的预计算结果
    ///
    /// `_rs1_val` 和 `_rs2_val` 参数为了 API 兼容性被接受
    ///（它们在真实 ALU 中会是转发的操作数值），但不被使用
    /// 因为 Trace 已经包含了结果。
    pub fn compute(record: &TraceRecord, _rs1_val: u64, _rs2_val: u64) -> u64 {
        match record.op_type {
            // 内存指令：ALU 计算有效地址。
            // Trace 提供实际 CPU 使用的地址。
            OpType::Load | OpType::Store => record.mem_addr,

            // JAL / JALR：ALU 计算返回地址 (pc + 4)。
            OpType::Jump => record.pc + 4,

            // IntAlu、分支、系统：使用 Trace 提供的结果。
            _ => record.rd_val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{OpType, TraceRecord};

    fn make_record(pc: u64, op_type: OpType, mem_addr: u64, rd_val: u64) -> TraceRecord {
        TraceRecord {
            pc,
            inst: 0,
            rs1: 0,
            rs2: 0,
            rd: 1,
            op_type,
            mem_addr,
            rd_val,
        }
    }

    #[test]
    fn test_alu_int_uses_rd_val() {
        let rec = make_record(0x100, OpType::IntAlu, 0, 42);
        assert_eq!(Alu::compute(&rec, 10, 32), 42);
    }

    #[test]
    fn test_alu_load_uses_mem_addr() {
        let rec = make_record(0x100, OpType::Load, 0x8000_0000, 99);
        assert_eq!(Alu::compute(&rec, 0, 0), 0x8000_0000);
    }

    #[test]
    fn test_alu_store_uses_mem_addr() {
        let rec = make_record(0x100, OpType::Store, 0x1000, 0);
        assert_eq!(Alu::compute(&rec, 0, 0), 0x1000);
    }

    #[test]
    fn test_alu_jump_uses_pc_plus_4() {
        let rec = make_record(0x200, OpType::Jump, 0, 0);
        assert_eq!(Alu::compute(&rec, 0, 0), 0x204);
    }

    #[test]
    fn test_alu_branch_uses_rd_val() {
        let rec = make_record(0x100, OpType::Branch, 0, 1); // taken=1
        assert_eq!(Alu::compute(&rec, 0, 0), 1);
    }
}
