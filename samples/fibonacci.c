typedef struct {
    void (*print)(const char* str, unsigned long len);
    unsigned long (*get_ticks)(void);
    void (*get_memory)(unsigned long* total, unsigned long* heap);
} MourosApi;

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

static void my_puts(const MourosApi* api, const char* s) {
    unsigned long len = 0;
    while (s[len]) len++;
    api->print(s, len);
}

int _start(const MourosApi* api) {
    my_puts(api, "=== Fibonacci & Prime Number Generator ===\n");
    my_puts(api, "First 15 Fibonacci Numbers:\n  ");

    unsigned long a = 0, b = 1;
    for (int i = 0; i < 15; i++) {
        print_num(api, a);
        my_puts(api, " ");
        unsigned long next = a + b;
        a = b;
        b = next;
    }
    my_puts(api, "\n\nPrime Numbers up to 50:\n  ");

    for (int n = 2; n <= 50; n++) {
        int is_prime = 1;
        for (int d = 2; d * d <= n; d++) {
            if (n % d == 0) {
                is_prime = 0;
                break;
            }
        }
        if (is_prime) {
            print_num(api, n);
            my_puts(api, " ");
        }
    }
    my_puts(api, "\n\nExecution finished successfully.\n");
    return 0;
}
