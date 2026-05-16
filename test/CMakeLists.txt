# 查找 gtest（通过 CMAKE_PREFIX_PATH 或系统路径）
find_package(GTest REQUIRED)

# ALU 单元测试
add_executable(alu_test
    alu_test.cc
)

target_link_libraries(alu_test
    trace_simulator_lib
    GTest::GTest
    GTest::Main
)

target_include_directories(alu_test PRIVATE ${CMAKE_SOURCE_DIR}/include)
gtest_discover_tests(alu_test)

# Hazard 单元测试
add_executable(hazard_test
    hazard_test.cc
)

target_link_libraries(hazard_test
    trace_simulator_lib
    GTest::GTest
    GTest::Main
)

target_include_directories(hazard_test PRIVATE ${CMAKE_SOURCE_DIR}/include)
gtest_discover_tests(hazard_test)

# Simulator 集成测试
add_executable(simulator_test
    simulator_test.cc
)

target_link_libraries(simulator_test
    trace_simulator_lib
    GTest::GTest
    GTest::Main
)

target_include_directories(simulator_test PRIVATE ${CMAKE_SOURCE_DIR}/include)
gtest_discover_tests(simulator_test)
