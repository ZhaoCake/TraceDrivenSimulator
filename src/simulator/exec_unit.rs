/// 建模执行单元的时序行为。
///
/// Trace 驱动模拟器中，执行单元不执行真实计算（Trace 提供结果）。
/// 它仅建模结果何时可用——即执行延迟与结构冒险检测。
///
/// 经典五级流水线中，所有单元延迟=1 且完全流水化（每周期可接收新指令）。
///
/// 未来扩展：
///   - 乘法器：延迟=3，流水化=true
///   - 除法器：延迟=20，非流水化（结构冒险）
#[derive(Debug, Clone)]
pub struct ExecUnit {
    /// 人类可读名称，用于调试/统计
    pub name: String,
    /// 从发射到结果可用的周期数
    pub latency: u32,
    /// 此单元是否每周期可接收新指令
    pub pipelined: bool,
    /// 单元变为空闲的周期号（非流水化单元）
    busy_until: u64,
}

impl ExecUnit {
    /// 创建新的执行单元。
    pub fn new(name: &str, latency: u32, pipelined: bool) -> Self {
        ExecUnit {
            name: name.to_string(),
            latency,
            pipelined,
            busy_until: 0,
        }
    }

    /// 创建默认整数 ALU 单元（1 周期，完全流水化）。
    pub fn default_int_alu() -> Self {
        Self::new("IntAlu", 1, true)
    }

    /// 创建默认加载/存储单元（1 周期，完全流水化）。
    pub fn default_lsu() -> Self {
        Self::new("LSU", 1, true)
    }

    /// 创建默认分支单元（1 周期，完全流水化）。
    pub fn default_branch() -> Self {
        Self::new("Branch", 1, true)
    }

    /// 检查单元本周期能否接收新指令。
    /// 流水化单元始终返回 true（无结构冒险）。
    /// 非流水化单元仅在空闲时返回 true。
    pub fn is_ready(&self, current_cycle: u64) -> bool {
        self.pipelined || current_cycle >= self.busy_until
    }

    /// 为从指定周期开始的指令预留此单元。
    /// 返回结果可用的周期号。
    /// 流水化单元：结果在 current_cycle + 1 可用（下一周期）。
    /// 非流水化单元：结果在 current_cycle + latency 可用。
    pub fn reserve(&mut self, current_cycle: u64) -> u64 {
        let ready_cycle = current_cycle + self.latency as u64;
        if !self.pipelined {
            self.busy_until = ready_cycle;
        }
        ready_cycle
    }

    /// 释放单元（非流水化单元在结果被消费后调用）。
    pub fn release(&mut self) {
        self.busy_until = 0;
    }
}
