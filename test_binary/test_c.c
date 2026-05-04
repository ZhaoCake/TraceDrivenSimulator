// 简单的斐波那契数列与冒泡排序，提供足够的指令多样性和计算复杂度供 Trace 模拟验证
int fib(int n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

void bubble_sort(int arr[], int n) {
    for (int i = 0; i < n - 1; i++) {
        for (int j = 0; j < n - i - 1; j++) {
            if (arr[j] > arr[j + 1]) {
                int temp = arr[j];
                arr[j] = arr[j + 1];
                arr[j + 1] = temp;
            }
        }
    }
}

int main() {
    volatile int result = fib(10);
    
    int data[] = { 10, 3, 5, 2, 7, 9, 8, 1, 4, 6 };
    bubble_sort(data, 10);
    
    return 0;
}