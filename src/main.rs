use std::fs::File;
use std::io::{self, Read};
use std::mem;
use std::path::Path;

use trace_simulator::trace::TraceRecordRaw;
use trace_simulator::simulator::Simulator;

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