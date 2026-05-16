#pragma once

#include <cstdint>

/// 指令类型的枚举定义
/// 涵盖了基础的 RISC-V 操作类别
enum class OpType : uint8_t {
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
};

/// 将原始的 u8 值转换为 OpType 枚举
/// 用于解析二进制 Trace 文件中的指令类型字段
OpType op_type_from_u8(uint8_t val);

/// 原始二进制格式中直接对齐映射的数据结构
/// 用于直接从文件中高效进行 mmap 或 read 内存拷贝
struct __attribute__((packed)) TraceRecordRaw {
    /// 指令所在程序计数器 (PC)
    uint64_t pc;
    /// 指令机器码原始值
    uint32_t inst;
    /// 源操作数寄存器1索引
    uint8_t rs1;
    /// 源操作数寄存器2索引
    uint8_t rs2;
    /// 目标操作数寄存器索引
    uint8_t rd;
    /// 指令大类的快速枚举表示 (原始 u8 格式)
    uint8_t op_type;
    /// 访存地址真值，在Load/Store时有效
    uint64_t mem_addr;
    /// 回写目标寄存器的真值，用于体系结构状态快速对齐
    uint64_t rd_val;
};

/// 在模拟器内流转和处理的格式化轨迹记录结构体
struct TraceRecord {
    /// 指令所在程序计数器 (PC)
    uint64_t pc;
    /// 指令机器码原始值
    uint32_t inst;
    /// 源操作数寄存器1索引
    uint8_t rs1;
    /// 源操作数寄存器2索引
    uint8_t rs2;
    /// 目标操作数寄存器索引
    uint8_t rd;
    /// 指令大类的枚举表示
    OpType op_type;
    /// 访存地址真值，在Load/Store时有效
    uint64_t mem_addr;
    /// 回写目标寄存器的真值，用于体系结构状态快速对齐
    uint64_t rd_val;

    /// 将从文件直接映射出的 raw record 转换为结构化的 TraceRecord
    static TraceRecord from_raw(const TraceRecordRaw& raw);
};
