use crate::trace::{OpType, TraceRecord};
use super::Simulator;
use super::latches::{IfIdLatch, IdExLatch, ExMemLatch, MemWbLatch};
use super::hazard::HazardUnit;
use super::alu::Alu;

impl Simulator {
    /// 将全部 Trace 记录加载到模拟器的输入队列。
    pub fn load_trace(&mut self, records: Vec<TraceRecord>) {
        self.trace = records.into();
    }

    /// 判断流水线是否已全部排空（无待处理指令，所有锁存器均为空）。
    pub fn is_done(&self) -> bool {
        self.trace.is_empty()
            && self.fd_latch.is_none()
            && self.de_latch.is_none()
            && self.em_latch.is_none()
            && self.mw_latch.is_none()
    }

    /// 两相周期推进：快照 → 组合逻辑 → 时序逻辑。
    ///
    /// 第一相（快照）：捕获周期开始时的不可变流水线状态。
    ///
    /// 第二相（组合逻辑）：HazardUnit 检测 Load-Use 冒险
    ///   （纯函数，无状态）。Alu 与 LSU 计算待完成结果。
    ///
    /// 第三相（时序逻辑）：按反向顺序提交流水线推进
    ///   （WB → MEM → EX → ID → IF）。反向顺序实现同周期
    ///   EX→ID 结果转发：ID 阶段读取刚更新的 EM/MW 锁存器。
    ///   HazardResult 控制停顿（冻结 IF，向 ID 插入气泡）。
    pub fn step_cycle(&mut self) {
        // ── 第一相（快照）──
        let snap = self.snapshot();

        // ── 第二相（组合逻辑）──
        let hazard = HazardUnit::resolve(&snap);

        // ── 第三相（时序逻辑）：反向提交实现同周期转发 ──

        // WB：将 MEM/WB 锁存器的结果写回寄存器文件。
        // 使用快照（克隆），因为 WB 读取不可变视图。
        if let Some(ref mw) = snap.mw {
            if mw.record.rd != 0 {
                let value = if mw.record.op_type == OpType::Load {
                    mw.mem_result
                } else {
                    mw.alu_result
                };
                self.registers[mw.record.rd as usize] = value;
            }
            self.inst_count += 1;
            *self.op_stats.entry(mw.record.op_type).or_insert(0) += 1;
        }

        // MEM：推进 EX/MEM → MEM/WB。Load 从 Trace 中获取访存结果。
        self.mw_latch = self.em_latch.take().map(|em| {
            let mem_result = if em.record.op_type == OpType::Load {
                em.record.rd_val
            } else {
                0
            };
            MemWbLatch::new(em.record, em.alu_result, mem_result)
        });

        // EX：推进 ID/EX → EX/MEM。使用 Alu::compute 进行结果路由。
        self.em_latch = self.de_latch.take().map(|de| {
            let alu_result = Alu::compute(&de.record, de.rs1_val, de.rs2_val);
            ExMemLatch::new(de.record, alu_result)
        });

        // ID：推进 IF/ID → ID/EX（含转发），应用 HazardResult。
        if hazard.stall_id || hazard.stall_if {
            // Load-Use 冒险：冻结 FD 锁存器，向 DE 插入气泡。
            self.stall = true;
            self.stall_count += 1;
            self.de_latch = None;
            // fd_latch 保持冻结（不消耗）
        } else {
            // 清除上一周期的停顿标志。
            self.stall = false;
            // 推进 fd → de（含操作数转发）。
            self.de_latch = self.fd_latch.take().map(|fd| {
                let rs1_val = self.forward_value(fd.record.rs1);
                let rs2_val = self.forward_value(fd.record.rs2);
                IdExLatch::new(fd.record, rs1_val, rs2_val)
            });
        }

        // IF：从 Trace 队列取指。
        if !self.stall && self.fd_latch.is_none() {
            self.fd_latch = self.trace.pop_front().map(|r| IfIdLatch::new(r));
        }

        self.cycle += 1;
    }

    /// 组合转发网络：解析源寄存器的值。
    ///
    /// 读取刚更新的 em_latch 和 mw_latch（在 EX/MEM 提交之后）
    /// 以建模 EX 到 ID 的同周期结果转发。
    ///
    /// 优先级（硬件转发多路选择器顺序）：
    ///   1. EX/MEM 的 alu_result（最近结果，Load 除外 — 数据尚未就绪）
    ///   2. MEM/WB 的 mem_result（Load）或 alu_result（非 Load）
    ///   3. 寄存器文件（默认）
    fn forward_value(&mut self, rs: u8) -> u64 {
        if rs == 0 {
            return 0;
        }
        // 优先级 1：EX/MEM（最近的非 Load 结果）
        if let Some(ref em) = self.em_latch {
            if em.record.rd == rs && em.record.op_type != OpType::Load {
                self.forward_count += 1;
                return em.alu_result;
            }
        }
        // 优先级 2：MEM/WB
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

    /// 打印模拟统计汇总日志。
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
