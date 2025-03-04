
| Name | Description | Default value | Allowed value |
|------|-------------|---------------|---------------|
|**ESP_HAL_EMBASSY_CONFIG_LOW_POWER_WAIT**|Enables the lower-power wait if no tasks are ready to run on the thread-mode executor. This allows the MCU to use less power if the workload allows. Recommended for battery-powered systems. May impact analog performance.|true|-
|**ESP_HAL_EMBASSY_CONFIG_TIMER_QUEUE**|<p>The flavour of the timer queue provided by this crate. Accepts one of `single-integrated`, `multiple-integrated` or `generic`. Integrated queues require the `executors` feature to be enabled.</p><p>If you use embassy-executor, the `single-integrated` queue is recommended for ease of use, while the `multiple-integrated` queue is recommended for performance. The `multiple-integrated` option needs one timer per executor.</p><p>The `generic` queue allows using embassy-time without the embassy executors.</p>|single-integrated|-
|**ESP_HAL_EMBASSY_CONFIG_GENERIC_QUEUE_SIZE**|The capacity of the queue when the `generic` timer queue flavour is selected.|64|Positive integer
