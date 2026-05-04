use std::fs::File;
use std::io::{self, Read};
use std::mem;
use std::path::Path;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    Unknown = 0,
    IntAlu = 1,
    Load = 2,
    Store = 3,
    Branch = 4,
    Jump = 5,
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

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TraceRecordRaw {
    pub pc: u64,
    pub inst: u32,
    pub rs1: u8,
    pub rs2: u8,
    pub rd: u8,
    pub op_type: u8,
    pub mem_addr: u64,
    pub rd_val: u64,
}

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

pub struct Simulator {
    pub cycle: u64,
    // Add architectural state here later
    // pub regs: [u64; 32],
}

impl Simulator {
    pub fn new() -> Self {
        Simulator { cycle: 0 }
    }

    pub fn step(&mut self, record: &TraceRecord) {
        self.cycle += 1;
        
        // 预留接口：依赖检查 (Dependency Check)
        self.check_dependencies(record);

        // 预留接口：时序模拟 (Timing/Pipeline Simulation)
        self.simulate_timing(record);

        // 顺序执行输出，只做验证
        println!("Cycle {:04} | PC: 0x{:016x} | OP: {:?} | RS1: x{:02}, RS2: x{:02} | RD: x{:02} <- 0x{:x} | Mem: 0x{:x}", 
            self.cycle, record.pc, record.op_type, record.rs1, record.rs2, record.rd, record.rd_val, record.mem_addr);
    }

    fn check_dependencies(&self, _record: &TraceRecord) {
        // 在这里实现原分发、计分板等逻辑
        // 检查 rs1 和 rs2 是否 ready
    }

    fn simulate_timing(&self, _record: &TraceRecord) {
        // 在这里实现多级流水线的资源占用和访存延迟
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <trace_file>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let mut file = File::open(path)?;

    let record_size = mem::size_of::<TraceRecordRaw>();
    let mut buffer = vec![0u8; record_size];

    let mut sim = Simulator::new();

    println!("Starting simulation...");
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

    println!("Simulation finished. Total cycles: {}", sim.cycle);
    Ok(())
}