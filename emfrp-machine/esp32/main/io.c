#include "io.h"
#ifdef __ESP
#include "driver/uart.h"
#define ECHO_TEST_TXD 1
#define ECHO_TEST_RXD 3
#define ECHO_TEST_RTS (UART_PIN_NO_CHANGE)
#define ECHO_TEST_CTS (UART_PIN_NO_CHANGE)
#define ECHO_UART_PORT_NUM 0
#define ECHO_UART_BAUD_RATE 115200
#ifdef EMFRP_DEBUG
#include "esp_log.h"
#endif
#endif
#ifdef __ARDUINO
#include "arduino.h"
#endif
void setup_uart_with_bufsize(int buf_size)
{
#ifdef __ESP
    /* Configure parameters of an UART driver,
     * communication pins and install the driver */
    uart_config_t uart_config = {
        .baud_rate = ECHO_UART_BAUD_RATE,
        .data_bits = UART_DATA_8_BITS,
        .parity = UART_PARITY_DISABLE,
        .stop_bits = UART_STOP_BITS_1,
        .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
        .source_clk = UART_SCLK_DEFAULT,
    };
    int intr_alloc_flags = 0;

#if CONFIG_UART_ISR_IN_IRAM
    intr_alloc_flags = ESP_INTR_FLAG_IRAM;
#endif

    uart_driver_install(ECHO_UART_PORT_NUM, buf_size, 0, 0, NULL, intr_alloc_flags);
    uart_param_config(ECHO_UART_PORT_NUM, &uart_config);
    uart_set_pin(ECHO_UART_PORT_NUM, ECHO_TEST_TXD, ECHO_TEST_RXD, ECHO_TEST_RTS, ECHO_TEST_CTS);
#endif
#ifdef __ARDUINO
    Serial.begin(115200);
#endif
}
void uart_write(const char *buf, int len)
{
#ifdef __ESP
    uart_write_bytes(ECHO_UART_PORT_NUM, buf, len);
#endif
#ifdef __ARDUINO
    Serial.writeBytes(buf, len);
#endif
}
int uart_read(char *buf, int max_len)
{
#ifdef __ESP
    return uart_read_bytes(ECHO_UART_PORT_NUM, buf, max_len, 0);
#endif
#ifdef __ARDUINO
    return Serial.readBytes(buf, max_len);
#endif
    return -1;
}
#ifdef EMFRP_DEBUG
void dbg_int(const char *info, int n)
{
#ifdef __ESP
    ESP_LOGI("[DEBUG]", "%s = %d", info, n);
#endif
#ifdef __ARDUINO
    printf("[DEBUG] %s = %d\n", info, n);

#endif
}
#endif
void uart_flush_()
{
#ifdef __ESP
    uart_flush(ECHO_UART_PORT_NUM);
#endif
#ifdef __ARDUINO
    Serial.flush();
#endif
}
