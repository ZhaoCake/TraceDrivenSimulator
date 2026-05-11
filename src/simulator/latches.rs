use crate::trace::TraceRecord;

/// IF/ID 锁存器 — 仅包含指令记录（尚无动态值）
#[derive(Debug, Clone)]
pub struct IfIdLatch {
    pub record: TraceRecord,
}

/// ID/EX 锁存器 — 包含经转发网络解析的寄存器值
#[derive(Debug, Clone)]
pub struct IdExLatch {
    pub record: TraceRecord,
    pub rs1_val: u64,
    pub rs2_val: u64,
}

/// EX/MEM 锁存器 — 包含 ALU 计算结果
#[derive(Debug, Clone)]
pub struct ExMemLatch {
    pub record: TraceRecord,
    pub alu_result: u64,
}

/// MEM/WB 锁存器 — 包含 ALU 结果与访存结果
#[derive(Debug, Clone)]
pub struct MemWbLatch {
    pub record: TraceRecord,
    pub alu_result: u64,
    pub mem_result: u64,
}

/// 流水线各锁存器状态的不可变快照。
/// 供组合逻辑（HazardUnit、Alu、LSU）使用，使其操作于一致的流水线状态视图之上。
#[derive(Debug, Clone)]
pub struct PipelineSnapshot {
    pub fd: Option<IfIdLatch>,
    pub de: Option<IdExLatch>,
    pub em: Option<ExMemLatch>,
    pub mw: Option<MemWbLatch>,
    pub registers: [u64; 32],
    #[allow(dead_code)]
    pub cycle: u64,
}

impl IfIdLatch {
    pub fn new(record: TraceRecord) -> Self {
        IfIdLatch { record }
    }
}

impl IdExLatch {
    pub fn new(record: TraceRecord, rs1_val: u64, rs2_val: u64) -> Self {
        IdExLatch {
            record,
            rs1_val,
            rs2_val,
        }
    }
}

impl ExMemLatch {
    pub fn new(record: TraceRecord, alu_result: u64) -> Self {
        ExMemLatch { record, alu_result }
    }
}

impl MemWbLatch {
    pub fn new(record: TraceRecord, alu_result: u64, mem_result: u64) -> Self {
        MemWbLatch {
            record,
            alu_result,
            mem_result,
        }
    }
}

impl PipelineSnapshot {
    pub fn new(
        fd: Option<IfIdLatch>,
        de: Option<IdExLatch>,
        em: Option<ExMemLatch>,
        mw: Option<MemWbLatch>,
        registers: [u64; 32],
        cycle: u64,
    ) -> Self {
        PipelineSnapshot {
            fd,
            de,
            em,
            mw,
            registers,
            cycle,
        }
    }
}
