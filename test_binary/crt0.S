    .global _start
_start:
    # 设置栈指针
    la sp, __stack_top
    
    # 跳转到 C 语言的 main 函数
    call main
    
    # 发送正常退出信号给 Spike (tohost = 1)
    la t0, tohost
    li t1, 1
    sw t1, 0(t0)

.end:
    j .end