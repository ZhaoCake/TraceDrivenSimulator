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
        self.cycle += 1;
        self.inst_count += 1;

        // 指令大类分布统计
        *self.op_stats.entry(record.op_type).or_insert(0) += 1;
        
        // 预留扩展点:
        // 在这里进行真正的乱序 / 顺序前递、计分板资源占用的逻辑模拟
        self.check_dependencies(record);
        self.simulate_timing(record);
    }

    /// 微架构依赖检查预留接口（例如寄存器时延标记）
    fn check_dependencies(&self, _record: &TraceRecord) {
        // 在这里实现源分发、计分板等逻辑
    }

    /// 微架构流水线/功能单元资源模拟预留接口
    fn simulate_timing(&self, _record: &TraceRecord) {
        // 在这里实现多级流水线的资源占用和访存延迟
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
        assert_eq!(sim.cycle, 3);
        assert_eq!(sim.inst_count, 3);
        assert_eq!(*sim.op_stats.get(&OpType::IntAlu).unwrap_or(&0), 2);
        assert_eq!(*sim.op_stats.get(&OpType::Store).unwrap_or(&0), 1);
        assert_eq!(*sim.op_stats.get(&OpType::Load).unwrap_or(&0), 0);
    }
}