# 配置并构建项目
build:
    cmake -B build && cmake --build build

# 运行测试
test:
    ctest --test-dir build --output-on-failure

# 彻底清理构建目录
clean:
    rm -rf build
