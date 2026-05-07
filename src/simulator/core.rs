use crate::trace::TraceRecord;
use super::Simulator;

impl Simulator {
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
