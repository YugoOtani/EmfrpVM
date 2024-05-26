#include "config.h"
void setup_uart_with_bufsize(int buf_size);
void uart_write(const char *buf, int len);
void uart_flush_();
int uart_read(char *buf, int maxlen);
#ifdef EMFRP_DEBUG
void dbg_int(const char *info, int n);
#endif