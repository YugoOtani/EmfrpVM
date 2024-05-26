#include "config.h"
#ifndef INCLUDE_MACHINE
#define INCLUDE_MACHINE
#include "machine.h"
#endif
#ifndef INCLUDE_IO
#define INCLUDE_IO
#include "io.h"
#endif
#include "freertos/task.h"
#include "driver/gpio.h"
#include "driver/gptimer.h"
#include "sdkconfig.h"
#include <unistd.h>
#include "esp_task_wdt.h"

#define START() gpio_set_level(2, 1)
#define END() gpio_set_level(2, 0)

static volatile bool update_flag = false;
uint8_t buf[UART_BUF_SIZE];

void gpio16_input(value_t *v)
{
    v->num = gpio_get_level(16);
}

void gpio5_output(value_t *v)
{
    gpio_set_level(5, v->num != 0);
}
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
    ESP_ERROR_CHECK(gpio_install_isr_service(0));
}

static bool IRAM_ATTR interrupt(gptimer_handle_t timer, const gptimer_alarm_event_data_t *edata, void *user_data)
{
    // stop timer immediately
    gptimer_stop(timer);
    gptimer_set_raw_count(timer, 0);
    update_flag = true;
    return false;
}

static void setup_gptimer(gptimer_handle_t *gptimer)
{
    gptimer_config_t timer_config = {
        .clk_src = GPTIMER_CLK_SRC_DEFAULT,
        .direction = GPTIMER_COUNT_UP,
        .resolution_hz = 1000000, // 1MHz, 1 tick=1us
    };
    ESP_ERROR_CHECK(gptimer_new_timer(&timer_config, gptimer));

    gptimer_event_callbacks_t cbs = {
        .on_alarm = interrupt,
    };
    ESP_ERROR_CHECK(gptimer_register_event_callbacks(*gptimer, &cbs, NULL));

    ESP_ERROR_CHECK(gptimer_enable(*gptimer));

    gptimer_alarm_config_t alarm_config1 = {
        .alarm_count = 500000, // period = 5s
    };
    ESP_ERROR_CHECK(gptimer_set_alarm_action(*gptimer, &alarm_config1));
}

void app_main(void)
{
    emfrp_result_t res;
    emfrp_machine_t em = {0};
    int data_len;
    gptimer_handle_t gptimer = NULL;
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size = esp_get_free_heap_size(); // 300556
#endif

    if (emfrp_init(&em, 1, 1) != EMFRP_OK) // 564
        goto err;
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size1 = esp_get_free_heap_size(); // 299992
#endif
    setup_uart_with_bufsize(UART_BUF_SIZE); // 2056
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size3 = esp_get_free_heap_size(); // 297936
#endif
    setup_gptimer(&gptimer); // 228
    gptimer_start(gptimer);
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size4 = esp_get_free_heap_size(); // 297708
#endif
    setup_gpio(); // 356byte
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size5 = esp_get_free_heap_size(); // 297352
#endif
    emfrp_add_input_node(&em, emfrp_false(), gpio16_input);
    emfrp_add_output_node(&em, emfrp_false(), gpio5_output);
#ifdef EMFRP_MEASURE_HEAP
    const uint32_t initial_heap_size2 = esp_get_free_heap_size();
#endif
    int read_len = 0, tmp;

#ifdef EMFRP_MEASURE_HEAP
    dbg_int("initial", initial_heap_size);
    dbg_int("initial2", initial_heap_size2);
#endif

    while (1)
    {
    begin_loop:
#ifdef EMFRP_MEASURE_HEAP
        dbg_int("initial", initial_heap_size);
        dbg_int("initial1", initial_heap_size1);
        dbg_int("initial3", initial_heap_size3);
        dbg_int("initial4", initial_heap_size4);
        dbg_int("initial5", initial_heap_size5);
        dbg_int("initial2", initial_heap_size2);
#endif
        if (update_flag)
        {
            update_flag = false;
            START();
            res = emfrp_update(&em);
            END();
            if (res == EMFRP_OUTOF_MEMORY)
                goto err;
#ifdef EMFRP_MEASURE_HEAP
            dbg_int("after update", esp_get_free_heap_size());
#endif

            gptimer_start(gptimer);
        }
        tmp = uart_read((char *)&buf[read_len], UART_BUF_SIZE - read_len);
        if (tmp == -1)
            goto begin_loop;
        read_len += tmp;
        if (read_len >= 2)
        {
            data_len = buf[0] + ((int)buf[1] << 8);
            while (read_len < data_len + 2)
            {
                tmp = uart_read((char *)&buf[read_len], UART_BUF_SIZE - read_len);
                if (tmp == -1)
                    goto begin_loop;
                read_len += tmp;
            }
            read_len = 0;
            res = emfrp_new_bytecode(&em, data_len, buf + 2);
            if (res == EMFRP_OUTOF_MEMORY)
            {
                goto err;
            }
            else
            {
                goto begin_loop;
            }
        }
        vTaskDelay(pdMS_TO_TICKS(100));
    }
err:
    while (1)
    {
        vTaskDelay(100);
    }
}
