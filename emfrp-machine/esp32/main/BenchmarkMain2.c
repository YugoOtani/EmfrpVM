#include "Benchmark.h"
#include "driver/gpio.h"
#include "freertos/FreeRTOS.h"
#include "esp_log.h"

void setup_gpio(void)
{
  gpio_config_t g = {0};
  g.intr_type = GPIO_INTR_DISABLE;
  g.mode = GPIO_MODE_OUTPUT;
  g.pin_bit_mask = 1ULL << 5 | 1ULL << 2; // GPIO5 and 2
  g.pull_down_en = 0;
  g.pull_up_en = 0;
  gpio_config(&g);
  g.intr_type = GPIO_INTR_DISABLE;
  g.pin_bit_mask = 1ULL << 16; // GPIO16
  g.mode = GPIO_MODE_INPUT;
  gpio_config(&g);
  gpio_install_isr_service(0);
}
void Input(int *gpio16)
{
  *gpio16 = gpio_get_level(16);
}
void Output(int *gpio5)
{
  gpio_set_level(5, *gpio5 != 0);
}
void app_main(void)
{
  setup_gpio();
  ESP_LOGI("HEAP", "%lu", esp_get_free_heap_size());
  ActivateBenchmark();
}
