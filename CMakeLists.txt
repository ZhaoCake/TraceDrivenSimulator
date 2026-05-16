cmake_minimum_required(VERSION 3.14)
project(TraceDrivenSimulator LANGUAGES CXX)

# C++17 标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# 生成 compile_commands.json 供 clangd 使用
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ── 库目标：所有模拟器模块 ──
add_library(trace_simulator_lib STATIC
    src/trace.cc
    src/simulator/exec_unit.cc
    src/simulator/alu.cc
    src/simulator/lsu.cc
    src/simulator/latches.cc
    src/simulator/hazard.cc
    src/simulator/simulator.cc
)

target_include_directories(trace_simulator_lib PUBLIC
    ${CMAKE_SOURCE_DIR}/include
)

# ── 可执行文件目标 ──
add_executable(trace_simulator
    src/main.cc
)

target_link_libraries(trace_simulator PRIVATE trace_simulator_lib)

# ── 测试 ──
enable_testing()
add_subdirectory(test)
