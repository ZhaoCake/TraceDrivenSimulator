#include <iostream>
#include <fstream>
#include <vector>
#include <cstring>
#include "trace.h"
#include "simulator/simulator.h"

/// 主启动函数：解析输入 Trace 文件并将其加载至流水线模拟器，
/// 以周期驱动方式推进流水线直至全部排空
int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "用法: " << argv[0] << " <trace二进制文件>" << std::endl;
        return 1;
    }

    const char* path = argv[1];
    std::ifstream file(path, std::ios::binary);
    if (!file.is_open()) {
        std::cerr << "无法打开文件: " << path << std::endl;
        return 1;
    }

    // ── 第一步：将全部 Trace 记录读入内存 ──
    std::cout << "[*] 读取 Trace 二进制文件..." << std::endl;
    std::vector<TraceRecord> records;
    TraceRecordRaw raw;

    while (file.read(reinterpret_cast<char*>(&raw), sizeof(TraceRecordRaw))) {
        records.push_back(TraceRecord::from_raw(raw));
    }

    // 处理最后一条不完整的记录（如有）
    if (file.gcount() > 0) {
        // 忽略不完整的记录，打印警告
        std::cerr << "警告: 最后一条 Trace 记录不完整，已忽略" << std::endl;
    }

    std::cout << "[*] 成功读取 " << records.size() << " 条 Trace 记录" << std::endl;

    // ── 第二步：加载至模拟器并按周期推进流水线 ──
    Simulator sim;
    sim.load_trace(records);

    std::cout << "[*] 开始按流水线周期推演..." << std::endl;
    while (!sim.is_done()) {
        sim.step_cycle();
    }

    std::cout << "[*] 流水线推演完毕。" << std::endl;
    sim.print_statistics();
    return 0;
}
