use crate::trace::OpType;
use super::Simulator;

impl Simulator {
    /// 写回阶段：将 MEM/WB 锁存器中的结果写回寄存器文件
    pub fn stage_wb(&mut self) {
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
    pub fn stage_mem(&mut self) {
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
    pub fn stage_ex(&mut self) {
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
    pub fn stage_id(&mut self) {
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
    pub fn stage_if(&mut self) {
        if self.stall {
            return;
        }
        if self.fd_latch.is_some() {
            return;
        }
        self.fd_latch = self.trace.pop_front().map(|record| super::Latch::new(record));
    }

    /// 转发网络：根据源寄存器号 rs 选择最新的数据来源
    ///
    /// 优先级（由高到低）：
    /// 1. EX/MEM 锁存器的 alu_result（前一条指令的 ALU 结果）
    ///    ⚠ Load 在 EX/MEM 时数据尚未加载，跳过
    /// 2. MEM/WB 锁存器的 mem_result（Load 刚取回的数据）或 alu_result
    /// 3. 寄存器文件（默认）
    ///
    /// 返回 rs 对应的最新操作数值
    pub fn forward(&mut self, rs: u8) -> u64 {
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
}
