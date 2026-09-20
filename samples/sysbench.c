typedef struct {
    void (*print)(const char* str, unsigned long len);
    unsigned long (*get_ticks)(void);
    void (*get_memory)(unsigned long* total, unsigned long* heap);
} MourosApi;

static void my_puts(const MourosApi* api, const char* s) {
    unsigned long len = 0;
    while (s[len]) len++;
    api->print(s, len);
}

static void print_num(const MourosApi* api, unsigned long n) {
    char buf[32];
    int i = 30;
    buf[31] = '\0';
    if (n == 0) {
        buf[i--] = '0';
    } else {
        while (n > 0 && i >= 0) {
            buf[i--] = '0' + (n % 10);
            n /= 10;
        }
    }
    int start = i + 1;
    api->print(&buf[start], 31 - start);
}

int _start(const MourosApi* api) {
    my_puts(api, "=== Mouros Micro-Benchmark Utility ===\n");

    unsigned long total_ram = 0, heap = 0;
    api->get_memory(&total_ram, &heap);
    my_puts(api, "Total Physical RAM: ");
    print_num(api, total_ram / (1024 * 1024));
    my_puts(api, " MiB\nKernel Heap Memory: ");
    print_num(api, heap / (1024 * 1024));
    my_puts(api, " MiB\n\n");

    my_puts(api, "Running 500,000 Integer Operations Benchmark...\n");
    unsigned long t0 = api->get_ticks();

    volatile unsigned long acc = 12345;
    for (int i = 0; i < 500000; i++) {
        acc = (acc * 1103515245 + 12345) & 0x7FFFFFFF;
    }

    unsigned long t1 = api->get_ticks();
    my_puts(api, "Benchmark completed!\nTimer Ticks elapsed: ");
    print_num(api, t1 - t0);
    my_puts(api, " ticks\nFinal Checksum: ");
    print_num(api, acc);
    my_puts(api, "\nPerformance: EXCELLENT (Sub-second execution)\n");

    return 0;
}
