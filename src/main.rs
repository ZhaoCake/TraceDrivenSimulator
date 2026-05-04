use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::mem;
use std::path::Path;

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

/// 性能仿真器核心结构体
/// 集成了执行状态推进、计分板预留以及基础指令吞吐量统计功能
pub struct Simulator {
    /// 模拟推演至目前花费的周期总数
    pub cycle: u64,
    /// 成功执行并提交的指令数目
    pub inst_count: u64,
    /// 按照操作类别进行的分布计数器 (指令混例统计)
    pub op_stats: HashMap<OpType, u64>,
    // TODO: 在这里添加你的微架构记分板、各类部件缓冲等时序推演状态队列
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
        // 在这里实现原分发、计分板等逻辑
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
            println!("  - {:<10}: {count:>8} 条 ({percentage:5.2}%)", format!("{:?}", op));
        }
        println!("========================================");
    }
}

/// 主启动函数：解析输入文件并将 Trace 加载到模拟器
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: {} <trace二进制文件>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let mut file = File::open(path)?;

    let record_size = mem::size_of::<TraceRecordRaw>();
    let mut buffer = vec![0u8; record_size];

    let mut sim = Simulator::new();

    println!("[*] 开始根据获取到的执行 Trace 流进行架构时序推演...");
    loop {
        match file.read_exact(&mut buffer) {
            Ok(_) => {
                let raw: TraceRecordRaw = unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const _) };
                let record = raw.into_record();
                sim.step(&record);
            }
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                break; // EOF
            }
            Err(e) => return Err(e),
        }
    }

    println!("[*] Trace 读取和推演完毕。");
    sim.print_statistics();
    Ok(())
}