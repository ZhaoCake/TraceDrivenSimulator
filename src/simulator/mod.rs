mod core;
mod pipeline;
#[cfg(test)]
mod tests;

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
}
