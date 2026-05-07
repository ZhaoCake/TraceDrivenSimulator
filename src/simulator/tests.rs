#[cfg(test)]
mod tests {
    use crate::trace::{OpType, TraceRecord};
    use super::super::Simulator;

    /// 构造一条 IntAlu 指令的 TraceRecord
    fn make_alu(pc: u64, rs1: u8, rs2: u8, rd: u8, rd_val: u64) -> TraceRecord {
        TraceRecord {
            pc, inst: 0,
            rs1, rs2, rd,
            op_type: OpType::IntAlu,
            mem_addr: 0,
            rd_val,
        }
    }

    /// 构造一条 Load 指令的 TraceRecord
    fn make_load(pc: u64, rs1: u8, rd: u8, rd_val: u64) -> TraceRecord {
        TraceRecord {
            pc, inst: 0,
            rs1, rs2: 0, rd,
            op_type: OpType::Load,
            mem_addr: 0x8000_0000,
            rd_val,
        }
    }

    /// 构造一条 Store 指令的 TraceRecord
    fn make_store(pc: u64, rs1: u8, rs2: u8, mem_addr: u64) -> TraceRecord {
        TraceRecord {
            pc, inst: 0,
            rs1, rs2, rd: 0,
            op_type: OpType::Store,
            mem_addr,
            rd_val: 0,
        }
    }

    /// 构造一条 Jump 指令的 TraceRecord
    fn make_jump(pc: u64, rd: u8) -> TraceRecord {
        TraceRecord {
            pc,
            inst: 0,
            rs1: 0,
            rs2: 0,
            rd,
            op_type: OpType::Jump,
            mem_addr: 0,
            rd_val: 0, // JAL 返回地址由 EX 阶段计算为 pc+4
        }
    }

    /// 构造一条 Branch 指令的 TraceRecord
    fn make_branch(pc: u64, rs1: u8, rs2: u8) -> TraceRecord {
        TraceRecord {
            pc,
            inst: 0,
            rs1,
            rs2,
            rd: 0,
            op_type: OpType::Branch,
            mem_addr: 0,
            rd_val: 0,
        }
    }

    #[test]
    /// 验证基本流水线功能：指令正确流过各阶段并写回
    fn test_basic_pipeline_flow() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 1, 10),
            make_alu(0x104, 0, 0, 2, 20),
            make_alu(0x108, 0, 0, 3, 30),
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // 3 条指令在五级流水线中：5 周期填充 + 2 周期排空 = 7 周期
        assert_eq!(sim.cycle, 7);
        assert_eq!(sim.inst_count, 3);
        assert_eq!(sim.registers[1], 10);
        assert_eq!(sim.registers[2], 20);
        assert_eq!(sim.registers[3], 30);
    }

    #[test]
    /// 验证 ALU → ALU 转发：后续指令直接从 EX/MEM 取得操作数
    fn test_alu_forwarding() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 4, 5, 1, 42),  // x1 = 42
            make_alu(0x104, 1, 0, 2, 84),  // x2 = x1 + 0（依赖 x1）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // 转发应命中至少 1 次（第二条指令的 rs1 从 EX/MEM 获取）
        assert!(sim.forward_count >= 1);
        assert_eq!(sim.registers[1], 42);
        assert_eq!(sim.registers[2], 84);
        // 无 Load-Use 停顿
        assert_eq!(sim.stall_count, 0);
    }

    #[test]
    /// 验证 Load-Use 冒险检测与停顿：Load 后紧跟使用其结果的指令会停顿一周期
    fn test_load_use_stall() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_load(0x100, 4, 1, 99),    // lw x1, 0(x4) → x1 = 99
            make_alu(0x104, 1, 0, 2, 199), // add x2, x1, x0（依赖 Load 结果）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // 应发生 1 次 Load-Use 停顿
        assert_eq!(sim.stall_count, 1);
        assert_eq!(sim.registers[1], 99);
        assert_eq!(sim.registers[2], 199);
        // 停顿后通过 WB 转发或寄存器文件获取正确值
    }

    #[test]
    /// 验证连续无依赖指令的 IPC 接近 1.0
    fn test_independent_instructions_ipc() {
        let mut sim = Simulator::new();
        let n = 100;
        let records: Vec<TraceRecord> = (0..n)
            .map(|i| make_alu(i * 4, 0, 0, (i % 31 + 1) as u8, i))
            .collect();
        sim.load_trace(records);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // n 条指令：填充 4 + n 个周期（IF/ID/EX/MEM 填充 + 最后一个经过 WB）
        let expected_cycles = 4 + n;
        assert_eq!(sim.cycle, expected_cycles);
        assert_eq!(sim.inst_count, n);
        let ipc = sim.inst_count as f64 / sim.cycle as f64;
        println!("无依赖 IPC: {:.3}", ipc);
        assert!(ipc > 0.9);
    }

    #[test]
    /// 验证 Store 指令正确流经流水线且不写回寄存器
    fn test_store_pipeline() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 5, 0xDEAD),
            make_store(0x104, 5, 0, 0x1000),
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.inst_count, 2);
        // Store 使用了 ALU 的结果 x5，应有一次转发
        assert!(sim.forward_count >= 1);
    }

    #[test]
    /// 验证 ALU → ALU → ALU 转发链：连续三条依赖指令
    fn test_alu_forwarding_chain() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 1, 10),   // x1 = 10
            make_alu(0x104, 1, 0, 2, 20),   // x2 = x1 + 10（依赖 x1）
            make_alu(0x108, 2, 0, 3, 30),   // x3 = x2 + 10（依赖 x2）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.registers[1], 10);
        assert_eq!(sim.registers[2], 20);
        assert_eq!(sim.registers[3], 30);
        // 至少 2 次转发（inst1←inst0, inst2←inst1）
        assert!(sim.forward_count >= 2);
        assert_eq!(sim.stall_count, 0);
    }

    #[test]
    /// 验证 Load 结果隔一条指令后使用（无需停顿，MEM/WB 转发即可）
    fn test_load_result_forwarded_from_mwb() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_load(0x100, 4, 1, 99),      // lw x1, 0(x4) → x1 = 99
            make_alu(0x104, 0, 0, 5, 50),    // 独立 ALU（填充一个槽位）
            make_alu(0x108, 1, 0, 2, 199),   // add x2, x1, x0（隔一条使用 Load 结果）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // 无停顿：消费者与 Load 间隔一条指令，Load 已在 MEM/WB 就绪
        assert_eq!(sim.stall_count, 0);
        assert_eq!(sim.registers[1], 99);
        assert_eq!(sim.registers[2], 199);
        // 应有一次 MW→ID 转发
        assert!(sim.forward_count >= 1);
    }

    #[test]
    /// 验证连续两条 Load 后紧跟同时依赖两者的 ALU 指令
    ///
    /// 时序分析：当消费者 ALU 进入 ID 时，第一条 Load 已在 MEM/WB 可通过转发
    /// 直接提供数据；第二条 Load 在 EX/MEM 尚未取数，触发一次停顿。
    /// 因此总停顿次数为 1。
    fn test_back_to_back_load_use() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_load(0x100, 4, 1, 10),      // lw x1, 0(x4)
            make_load(0x104, 5, 2, 20),      // lw x2, 0(x5)
            make_alu(0x108, 1, 2, 3, 30),    // add x3, x1, x2（依赖两个 Load）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.registers[1], 10);
        assert_eq!(sim.registers[2], 20);
        assert_eq!(sim.registers[3], 30);
        // 仅第二次 Load 会触发停顿（第一条 Load 已在 MEM/WB 转发就绪）
        assert_eq!(sim.stall_count, 1);
    }

    #[test]
    /// 验证 Jump 指令：JAL 的返回地址 (pc+4) 正确写回 rd
    fn test_jump_link_writeback() {
        let mut sim = Simulator::new();
        let jump_pc = 0x100;
        sim.load_trace(vec![
            make_jump(jump_pc, 1), // jal x1, target → x1 = pc + 4
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.registers[1], jump_pc + 4);
        assert_eq!(sim.inst_count, 1);
    }

    #[test]
    /// 验证 Branch 指令正常流经流水线（不写回寄存器）
    fn test_branch_pipeline_flow() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 5, 5),
            make_branch(0x104, 5, 0), // beq x5, x0
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.inst_count, 2);
        // Branch 用到了 x5，应从转发获取
        assert!(sim.forward_count >= 1);
    }

    #[test]
    /// 验证 Store 指令的 rs2 值能从之前指令正确转发
    fn test_store_rs2_forwarding() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 5, 0xABCD),   // x5 = 0xABCD
            make_alu(0x104, 5, 0, 6, 0x5678),   // x6 = x5 + ... (依赖 x5)
            make_store(0x108, 10, 6, 0x8000),    // sw x6, 0(x10)（依赖 x6）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.inst_count, 3);
        // x5→x6 一次转发，x6→store 一次转发
        assert!(sim.forward_count >= 2);
        assert_eq!(sim.stall_count, 0);
        assert_eq!(sim.registers[5], 0xABCD);
        assert_eq!(sim.registers[6], 0x5678);
    }

    #[test]
    /// 验证 x0 寄存器始终为 0，即使有指令试图写 x0
    fn test_x0_always_zero() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 0, 999), // 试图写 x0
            make_alu(0x104, 0, 0, 0, 888), // 再次试图写 x0
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // x0 始终为 0
        assert_eq!(sim.registers[0], 0);
        // 但指令仍应被计数
        assert_eq!(sim.inst_count, 2);
    }

    #[test]
    /// 验证转发优先级：EX/MEM 优先于 MEM/WB（最近的结果优先）
    fn test_forwarding_priority_em_over_mw() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 3, 100),  // x3 = 100
            make_alu(0x104, 0, 0, 3, 200),  // x3 = 200（覆盖 x3）
            make_alu(0x108, 3, 0, 5, 300),  // x5 = x3 + ...（应取 EX/MEM 中的 200，而非 MEM/WB 中的 100）
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        // x5 应使用最近写入 x3 的值 200
        assert_eq!(sim.registers[3], 200);
        assert_eq!(sim.registers[5], 300);
        // 第三次指令应从 EX/MEM 转发（而非 MEM/WB）
        assert!(sim.forward_count >= 1);
    }

    #[test]
    /// 验证空 Trace 时 is_done 立即为 true
    fn test_empty_trace() {
        let sim = Simulator::new();
        assert!(sim.is_done());
        assert_eq!(sim.cycle, 0);
        assert_eq!(sim.inst_count, 0);
    }

    #[test]
    /// 验证混合指令类型的操作统计分布
    fn test_op_stats_distribution() {
        let mut sim = Simulator::new();
        sim.load_trace(vec![
            make_alu(0x100, 0, 0, 1, 10),
            make_load(0x104, 1, 2, 20),
            make_alu(0x108, 2, 0, 3, 30),
            make_store(0x10C, 3, 0, 0x8000),
            make_branch(0x110, 1, 2),
            make_jump(0x114, 5),
        ]);

        while !sim.is_done() {
            sim.step_cycle();
        }

        assert_eq!(sim.inst_count, 6);
        assert_eq!(sim.op_stats.get(&OpType::IntAlu), Some(&2));
        assert_eq!(sim.op_stats.get(&OpType::Load), Some(&1));
        assert_eq!(sim.op_stats.get(&OpType::Store), Some(&1));
        assert_eq!(sim.op_stats.get(&OpType::Branch), Some(&1));
        assert_eq!(sim.op_stats.get(&OpType::Jump), Some(&1));
    }
}
