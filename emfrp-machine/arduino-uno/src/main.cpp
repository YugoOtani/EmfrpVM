#include "config.h"
#include "machine.h"
static uint8_t buf[UART_BUF_SIZE];
static int read_len = 0;
static emfrp_machine_t em;

void gpio2_input(value_t *v)
{
    if (digitalRead(2) == HIGH)
    {
        v->num = 1;
    }
    else
    {
        v->num = 0;
    }
}

void gpio_output(value_t *v)
{
    digitalWrite(LED_BUILTIN, v->num ? HIGH : LOW);
}
void setup()
{
    Serial.begin(115200);
    pinMode(LED_BUILTIN, OUTPUT);
    emfrp_init(&em, 0, 1);
    // emfrp_add_input_node(&em, emfrp_int(0), gpio2_input);
    emfrp_add_output_node(&em, emfrp_int(0), gpio_output);
}

void loop()
{
    emfrp_result_t res;
    read_len += Serial.readBytes(&buf[read_len], UART_BUF_SIZE - read_len);
    if (read_len >= 2)
    {
        int data_len = (int)buf[0] + ((int)buf[1] << 8);
        while (read_len < data_len + 2)
        {
            read_len += Serial.readBytes(&buf[read_len], UART_BUF_SIZE - read_len);
        }
        read_len = 0;
        res = emfrp_new_bytecode(&em, data_len, &buf[2]);
        Serial.write((char *)&res, 1);
    }
    else
    {
        emfrp_update(&em);
    }
}