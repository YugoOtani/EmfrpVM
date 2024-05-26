#include <Arduino.h>
#include "machine.h"
#define BUF_SIZE 128
static uint8_t buf[BUF_SIZE];
static int read_len = 0;
static emfrp_machine em;

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

void gpio1_output(value_t *v)
{
  digitalWrite(1, v->num ? HIGH : LOW);
}
void setup()
{
  Serial.begin(115200);
  pinMode(2, INPUT);
  pinMode(1, OUTPUT);
  emfrp_init(&em, 1, 1);
  emfrp_add_input_node(&em, emfrp_int(0), gpio2_input);
  emfrp_add_output_node(&em, emfrp_int(0), gpio1_output);
}

void loop()
{
  read_len += Serial.readBytes(&buf[read_len], BUF_SIZE - read_len);
  if (read_len >= 4)
  {
    int data_len = (int)buf[0] + ((int)buf[1] << 8) + ((int)buf[2] << 16) + ((int)buf[3] << 24);
    while (read_len < data_len + 4)
    {
      read_len += Serial.readBytes(&buf[read_len], BUF_SIZE - read_len);
    }
    read_len = 0;
    emfrp_new_bytecode(&em, data_len, &buf[4]);
  }
  else
  {
    emfrp_update(&em);
  }
}
