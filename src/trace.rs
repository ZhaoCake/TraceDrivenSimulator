/// 指令类型的枚举定义
/// 涵盖了基础的 RISC-V 操作类别
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpType {
    /// 未知类型
    Unknown = 0,
    /// 算术逻辑运算指令 (如: add, sub, and, or 等)
    IntAlu = 1,
    /// 访存加载指令 (如: lw, lb 等)
    Load = 2,
    /// 访存存储指令 (如: sw, sb 等)
    Store = 3,
    /// 条件分支指令 (如: beq, bne 等)
    Branch = 4,
    /// 无条件跳转指令 (如: jal, jalr)
    Jump = 5,
    /// 系统调用或控制状态寄存器指令 (如: ecall, csrrw 等)
    System = 6,
}

impl From<u8> for OpType {
    fn from(val: u8) -> Self {
        match val {
            1 => OpType::IntAlu,
            2 => OpType::Load,
            3 => OpType::Store,
            4 => OpType::Branch,
            5 => OpType::Jump,
            6 => OpType::System,
            _ => OpType::Unknown,
        }
    }
}

/// 原始二进制格式中直接对齐映射的数据结构
/// 用于直接从文件中高效进行 mmap 或 read_exact 内存拷贝
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TraceRecordRaw {
    /// 指令所在程序计数器 (PC)
    pub pc: u64,
    /// 指令机器码原始值
    pub inst: u32,
    /// 源操作数寄存器1索引
    pub rs1: u8,
    /// 源操作数寄存器2索引
    pub rs2: u8,
    /// 目标操作数寄存器索引
    pub rd: u8,
    /// 指令大类的快速枚举表示
    pub op_type: u8,
    /// 访存地址真值，在Load/Store时有效
    pub mem_addr: u64,
    /// 回写目标寄存器的真值，用于体系结构状态快速对齐
    pub rd_val: u64,
}

/// 在模拟器内流转和处理的格式化轨迹记录结构体
#[derive(Debug, Clone)]
pub struct TraceRecord {
    pub pc: u64,
    pub inst: u32,
    pub rs1: u8,
    pub rs2: u8,
    pub rd: u8,
    pub op_type: OpType,
    pub mem_addr: u64,
    pub rd_val: u64,
}

impl TraceRecordRaw {
    /// 将从文件直接映射出的 raw record 转换为结构化的 TraceRecord
    pub fn into_record(self) -> TraceRecord {
        TraceRecord {
            pc: self.pc,
            inst: self.inst,
            rs1: self.rs1,
            rs2: self.rs2,
            rd: self.rd,
            op_type: OpType::from(self.op_type),
            mem_addr: self.mem_addr,
            rd_val: self.rd_val,
        }
    }
}