use std::fs::File;
use std::io::{self, Read};
use std::mem;
use std::path::Path;

use trace_simulator::trace::TraceRecordRaw;
use trace_simulator::simulator::Simulator;

/// 主启动函数：解析输入 Trace 文件并将其加载至流水线模拟器，
/// 以周期驱动方式推进流水线直至全部排空
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
    let mut records = Vec::new();

    // ── 第一步：将全部 Trace 记录读入内存 ──
    println!("[*] 读取 Trace 二进制文件...");
    loop {
        match file.read_exact(&mut buffer) {
            Ok(_) => {
                let raw: TraceRecordRaw =
                    unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const _) };
                records.push(raw.into_record());
            }
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }
    println!("[*] 成功读取 {} 条 Trace 记录", records.len());

    // ── 第二步：加载至模拟器并按周期推进流水线 ──
    let mut sim = Simulator::new();
    sim.load_trace(records);

    println!("[*] 开始按流水线周期推演...");
    while !sim.is_done() {
        sim.step_cycle();
    }

    println!("[*] 流水线推演完毕。");
    sim.print_statistics();
    Ok(())
}
