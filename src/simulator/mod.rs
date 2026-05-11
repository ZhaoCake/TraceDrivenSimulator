mod latches;
mod core;
mod exec_unit;
mod alu;
mod lsu;
mod hazard;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, VecDeque};
use crate::trace::{OpType, TraceRecord};
use latches::{IfIdLatch, IdExLatch, ExMemLatch, MemWbLatch, PipelineSnapshot};
use exec_unit::ExecUnit;
use lsu::Lsu;

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
    fd_latch: Option<IfIdLatch>,
    /// ID/EX：译码 → 执行
    de_latch: Option<IdExLatch>,
    /// EX/MEM：执行 → 访存
    em_latch: Option<ExMemLatch>,
    /// MEM/WB：访存 → 写回
    mw_latch: Option<MemWbLatch>,

    /// Load-Store Unit（建模访存时序）
    lsu: Lsu,

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
            lsu: Lsu::new(),
            registers: [0; 32],
            trace: VecDeque::new(),
            stall: false,
            stall_count: 0,
            forward_count: 0,
        }
    }

    /// 获取当前流水线状态的不可变快照，供组合逻辑（HazardUnit、Alu、LSU）使用
    fn snapshot(&self) -> PipelineSnapshot {
        PipelineSnapshot::new(
            self.fd_latch.clone(),
            self.de_latch.clone(),
            self.em_latch.clone(),
            self.mw_latch.clone(),
            self.registers,
            self.cycle,
        )
    }
}
