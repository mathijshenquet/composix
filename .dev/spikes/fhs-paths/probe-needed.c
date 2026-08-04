#include <stdio.h>

int cix_fhs_probe_value(void);

int main(void) {
    if (cix_fhs_probe_value() != 95) {
        return 1;
    }
    puts("fhs-needed-ok");
    return 0;
}
