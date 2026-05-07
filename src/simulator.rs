use std::collections::{HashMap, VecDeque};
use crate::trace::{OpType, TraceRecord};

/// 流水线阶段间锁存器
/// 保存当前指令的静态信息以及在流水线中逐级传递的动态计算值
#[derive(Debug, Clone)]
struct Latch {
    /// 该指令的 Trace 记录（静态信息：pc、rs1/rs2/rd、op_type 等）
    record: TraceRecord,
    /// ID 阶段读取的 rs1 操作数值（经转发网络选择）
    rs1_val: u64,
    /// ID 阶段读取的 rs2 操作数值（经转发网络选择）
    rs2_val: u64,
    /// EX 阶段计算出的 ALU 结果（非访存指令即为最终写回值）
    alu_result: u64,
    /// MEM 阶段从内存加载的值（仅 Load 指令有效）
    mem_result: u64,
}

impl Latch {
    /// 从 TraceRecord 创建新的锁存器，动态值初始化为 0
    fn new(record: TraceRecord) -> Self {
        Latch {
            record,
            rs1_val: 0,
            rs2_val: 0,
            alu_result: 0,
            mem_result: 0,
        }
    }
}

/// 性能仿真器核心结构体
/// 实现经典五级流水线（IF/ID/EX/MEM/WB），包含转发网络与 Load-Use 冒险停顿
pub struct Simulator {
    /// 模拟推演至目前花费的周期总数
    pub cycle: u64,
    /// 成功执行并提交的指令数目（在 WB 阶段计数）
    pub inst_count: u64,
    /// 按照操作类别进行的分布计数器（指令混例统计）
    pub op_stats: HashMap<OpType, u64>,

    // ── 流水线锁存器 ──
    /// IF/ID：取指 → 译码
    fd_latch: Option<Latch>,
    /// ID/EX：译码 → 执行
    de_latch: Option<Latch>,
    /// EX/MEM：执行 → 访存
    em_latch: Option<Latch>,
    /// MEM/WB：访存 → 写回
    mw_latch: Option<Latch>,

    /// 体系结构寄存器文件（32 个整数寄存器，x0 硬连线为 0）
    registers: [u64; 32],

    /// Trace 输入队列，按程序顺序存放待发射的指令
    trace: VecDeque<TraceRecord>,

    /// 流水线停顿标志：为 true 时 IF 冻结、ID 插入气泡
    stall: bool,

    /// Load-Use 冒险导致的停顿次数
    pub stall_count: u64,
    /// 转发网络命中次数（从流水线锁存器获取操作数而非寄存器文件）
    pub forward_count: u64,
}

impl Simulator {
    /// 构造一个新的空白模拟器对象
    pub fn new() -> Self {
        Simulator {
            cycle: 0,
            inst_count: 0,
            op_stats: HashMap::new(),
            fd_latch: None,
            de_latch: None,
            em_latch: None,
            mw_latch: None,
            registers: [0; 32],
            trace: VecDeque::new(),
            stall: false,
            stall_count: 0,
            forward_count: 0,
        }
    }

    /// 将全部 Trace 记录加载到模拟器的输入队列
    pub fn load_trace(&mut self, records: Vec<TraceRecord>) {
        self.trace = records.into();
    }

    /// 判断流水线是否已全部排空（无待发射指令，且所有锁存器为空）
    pub fn is_done(&self) -> bool {
        self.trace.is_empty()
            && self.fd_latch.is_none()
            && self.de_latch.is_none()
            && self.em_latch.is_none()
            && self.mw_latch.is_none()
    }

    /// 核心周期推进函数：按 WB → MEM → EX → ID → IF 的顺序逆向处理各阶段，
    /// 确保同一周期内写回的值能被译码阶段读取到
    pub fn step_cycle(&mut self) {
        self.stage_wb();
        self.stage_mem();
        self.stage_ex();
        self.stage_id();
        self.stage_if();
        self.cycle += 1;
    }

    // ── 五级流水线各阶段实现 ──

    /// 写回阶段：将 MEM/WB 锁存器中的结果写回寄存器文件
    fn stage_wb(&mut self) {
        if let Some(ref latch) = self.mw_latch {
            if latch.record.rd != 0 {
                // Load 指令写回内存加载值，其他指令写回 ALU 结果
                let value = if latch.record.op_type == OpType::Load {
                    latch.mem_result
                } else {
                    latch.alu_result
                };
                self.registers[latch.record.rd as usize] = value;
            }
            self.inst_count += 1;
            *self.op_stats.entry(latch.record.op_type).or_insert(0) += 1;
        }
    }

    /// 访存阶段：将 EX/MEM 锁存器推进至 MEM/WB，Load 指令在此获取访存结果
    fn stage_mem(&mut self) {
        let mut mw = self.em_latch.take();
        if let Some(ref mut latch) = mw {
            if latch.record.op_type == OpType::Load {
                // 从 Trace 中取得加载的真实值
                latch.mem_result = latch.record.rd_val;
            }
            // Store 指令无需额外处理（mem_addr 已在锁存器中）
        }
        self.mw_latch = mw;
    }

    /// 执行阶段：将 ID/EX 锁存器推进至 EX/MEM，在此计算 ALU 结果
    fn stage_ex(&mut self) {
        let mut em = self.de_latch.take();
        if let Some(ref mut latch) = em {
            match latch.record.op_type {
                OpType::Load | OpType::Store => {
                    // 访存指令：ALU 计算有效地址（Trace 已提供）
                    latch.alu_result = latch.record.mem_addr;
                }
                OpType::Jump => {
                    // JAL / JALR：alu_result = pc + 4（返回地址），供写回 rd 使用
                    latch.alu_result = latch.record.pc + 4;
                }
                _ => {
                    // IntAlu / Branch / System 等：来自 Trace 的真值
                    latch.alu_result = latch.record.rd_val;
                }
            }
        }
        self.em_latch = em;
    }

    /// 译码阶段：将 IF/ID 锁存器推进至 ID/EX，实现寄存器读取、转发网络、
    /// 以及 Load-Use 冒险检测与停顿
    fn stage_id(&mut self) {
        // 上周期停顿恢复：清除标志，本周期正常处理之前冻结的 FD
        if self.stall {
            self.stall = false;
        }

        // Load-Use 冒险检测：EX/MEM 中有 Load，且 FD 中的指令依赖其结果
        let mut hazard = false;
        if let Some(ref em) = self.em_latch {
            if em.record.op_type == OpType::Load && em.record.rd != 0 {
                if let Some(ref fd) = self.fd_latch {
                    if fd.record.rs1 == em.record.rd || fd.record.rs2 == em.record.rd {
                        hazard = true;
                    }
                }
            }
        }

        if hazard {
            // 停顿一周期：FD 锁存器冻结，ID 插入气泡
            self.stall = true;
            self.stall_count += 1;
            self.de_latch = None;
            return;
        }

        // 正常推进（含停顿恢复后推进），经转发网络读取操作数
        let mut de = self.fd_latch.take();
        if let Some(ref mut latch) = de {
            latch.rs1_val = self.forward(latch.record.rs1);
            latch.rs2_val = self.forward(latch.record.rs2);
        }
        self.de_latch = de;
    }

    /// 取指阶段：从 Trace 队列中取出下一条指令填入 IF/ID 锁存器，
    /// 若停顿信号有效或 FD 未空则保持
    fn stage_if(&mut self) {
        if self.stall {
            return;
        }
        if self.fd_latch.is_some() {
            return;
        }
        self.fd_latch = self.trace.pop_front().map(|record| Latch::new(record));
    }

    // ── 转发网络 ──

    /// 转发网络：根据源寄存器号 rs 选择最新的数据来源
    ///
    /// 优先级（由高到低）：
    /// 1. EX/MEM 锁存器的 alu_result（前一条指令的 ALU 结果）
    ///    ⚠ Load 在 EX/MEM 时数据尚未加载，跳过
    /// 2. MEM/WB 锁存器的 mem_result（Load 刚取回的数据）或 alu_result
    /// 3. 寄存器文件（默认）
    ///
    /// 返回 rs 对应的最新操作数值
    fn forward(&mut self, rs: u8) -> u64 {
        if rs == 0 {
            return 0; // x0 硬连线为 0
        }

        // 优先级 1：EX/MEM 阶段（最近的结果）
        // Load 指令在 EX/MEM 阶段数据尚未加载完成（alu_result 此时为地址），
        // 不能转发，需等待 MEM/WB 阶段通过 mem_result 转发
        if let Some(ref em) = self.em_latch {
            if em.record.rd == rs && em.record.op_type != OpType::Load {
                self.forward_count += 1;
                return em.alu_result;
            }
        }

        // 优先级 2：MEM/WB 阶段
        if let Some(ref mw) = self.mw_latch {
            if mw.record.rd == rs {
                self.forward_count += 1;
                return if mw.record.op_type == OpType::Load {
                    mw.mem_result
                } else {
                    mw.alu_result
                };
            }
        }

        // 优先级 3：寄存器文件
        self.registers[rs as usize]
    }

    /// 打印模拟统计汇总日志
    pub fn print_statistics(&self) {
        println!("\n=========== 性能模拟统计结果 ===========");
        println!("执行周期总计 (Trace Cycles) : {}", self.cycle);
        println!("处理指令条数 (Inst Count)   : {}", self.inst_count);
        println!("综合 IPC (Instr Per Cycle)  : {:.3}", (self.inst_count as f64) / (self.cycle as f64));
        println!("转发命中次数 (Forwards)     : {}", self.forward_count);
        println!("Load-Use 停顿次数 (Stalls)  : {}", self.stall_count);
        println!("-------- 基础指令分布 (Inst Mix) --------");
        for (op, count) in &self.op_stats {
            let percentage = (*count as f64) / (self.inst_count as f64) * 100.0;
            println!("  - {:<10}: {:>8} 条 ({:5.2}%)", format!("{:?}", op), count, percentage);
        }
        println!("========================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
