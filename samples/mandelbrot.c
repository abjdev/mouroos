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
    my_puts(api, "=== ASCII Mandelbrot Fractal Generator ===\n");
    const char chars[] = " .:-=+*#%@";

    for (int y = -10; y <= 10; y++) {
        char line[50];
        int col = 0;
        for (int x = -20; x <= 20; x++) {
            int cr = x * 100 / 20;
            int ci = y * 100 / 10;
            int zr = 0, zi = 0;
            int iter = 0;
            for (; iter < 9; iter++) {
                int zr2 = (zr * zr) / 100;
                int zi2 = (zi * zi) / 100;
                if (zr2 + zi2 > 400) break;
                zi = (2 * zr * zi) / 100 + ci;
                zr = zr2 - zi2 + cr;
            }
            line[col++] = chars[iter];
        }
        line[col++] = '\n';
        api->print(line, col);
    }
    my_puts(api, "\nMandelbrot render complete.\n");
    return 0;
}
