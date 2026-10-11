#include <stdlib.h>
int main(void) { volatile char *p = (char *)malloc(1); p[1] = 7; free((void *)p); return 0; }
