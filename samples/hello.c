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

int _start(const MourosApi* api) {
    my_puts(api, "=== Mouros ELF Process: Hello World ===\n");
    my_puts(api, "Running native 64-bit ELF binary in ring 0 context!\n");
    my_puts(api, "Process ID: 1042  |  Status: SUCCESS\n\n");
    return 0;
}
