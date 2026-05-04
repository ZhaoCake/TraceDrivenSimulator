use std::collections::HashMap;
use crate::trace::{OpType, TraceRecord};

/// 性能仿真器核心结构体
/// 集成了执行状态推进、计分板预留以及基础指令吞吐量统计功能
pub struct Simulator {
    /// 模拟推演至目前花费的周期总数
    pub cycle: u64,
    /// 成功执行并提交的指令数目
    pub inst_count: u64,
    /// 按照操作类别进行的分布计数器 (指令混例统计)
    pub op_stats: HashMap<OpType, u64>,
}

impl Simulator {
    /// 构造一个新的空白模拟器对象
    pub fn new() -> Self {
        Simulator {
            cycle: 0,
            inst_count: 0,
            op_stats: HashMap::new(),
        }
    }

    /// 核心推进函数（前进单个 Trace 指令）
    pub fn step(&mut self, record: &TraceRecord) {
        self.inst_count += 1;

        // 指令大类分布统计
        *self.op_stats.entry(record.op_type).or_insert(0) += 1;
        
        // 多周期推演：将指令逐步送入五个阶段进行处理
        // 后续改为流水线时，可以将这五个阶段操作并行作用于不同的指令
        self.stage_fetch(record);
        self.stage_decode(record);
        self.stage_execute(record);
        self.stage_memory(record);
        self.stage_writeback(record);
    }

    /// 取指阶段 (Instruction Fetch)
    fn stage_fetch(&mut self, _record: &TraceRecord) {
        // 在这里预留：依据 PC 抓取指令、模拟 I-Cache 延迟等
        self.cycle += 1;
    }

    /// 译码阶段 (Instruction Decode)
    fn stage_decode(&mut self, _record: &TraceRecord) {
        // 在这里预留：解析寄存器 rs1/rs2，检查数据相关性等
        self.cycle += 1;
    }

    /// 执行阶段 (Execute)
    fn stage_execute(&mut self, _record: &TraceRecord) {
        // 在这里预留：进行真正的 ALU 计算、计算分支结果与预测对比等
        self.cycle += 1;
    }

    /// 访存阶段 (Memory Access)
    fn stage_memory(&mut self, _record: &TraceRecord) {
        // 在这里预留：基于 mem_addr 模拟 D-Cache 延迟、计算访存阻塞等
        self.cycle += 1;
    }

    /// 写回阶段 (Write Back)
    fn stage_writeback(&mut self, _record: &TraceRecord) {
        // 在这里预留：更新记分板状态，解除结构冒险与数据冒险的标记等
        self.cycle += 1;
    }

    /// 完成执行打印汇总日志
    pub fn print_statistics(&self) {
        println!("\n=========== 性能模拟统计结果 ===========");
        println!("执行周期总计 (Trace Cycles) : {}", self.cycle);
        println!("处理指令条数 (Inst Count)   : {}", self.inst_count);
        println!("综合 IPC (Instr Per Cycle)  : {:.3}", (self.inst_count as f64) / (self.cycle as f64));
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

    #[test]
    fn test_simulator_step_and_stats() {
        let mut sim = Simulator::new();

        // 构造几个虚构的 TraceRecord 验证模拟器逻辑
        let trace_sequence = vec![
            TraceRecord {
                pc: 0x8000_0000, inst: 0x0000_0000,
                rs1: 0, rs2: 0, rd: 1, op_type: OpType::IntAlu,
                mem_addr: 0, rd_val: 10
            },
            TraceRecord {
                pc: 0x8000_0004, inst: 0x0000_0000,
                rs1: 1, rs2: 0, rd: 2, op_type: OpType::IntAlu,
                mem_addr: 0, rd_val: 20
            },
            TraceRecord {
                pc: 0x8000_0008, inst: 0x0000_0000,
                rs1: 2, rs2: 0, rd: 0, op_type: OpType::Store,
                mem_addr: 0x8000_1000, rd_val: 0
            }
        ];

        for record in &trace_sequence {
            sim.step(record);
        }

        // 测试基础统计指标
        assert_eq!(sim.cycle, 15);
        assert_eq!(sim.inst_count, 3);
        assert_eq!(*sim.op_stats.get(&OpType::IntAlu).unwrap_or(&0), 2);
        assert_eq!(*sim.op_stats.get(&OpType::Store).unwrap_or(&0), 1);
        assert_eq!(*sim.op_stats.get(&OpType::Load).unwrap_or(&0), 0);
    }
}