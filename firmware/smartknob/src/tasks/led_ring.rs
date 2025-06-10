#[embassy_executor::task]
pub async fn led_ring(mut led_pin: Output) {
    let mut led_ring = LedRing::new(led_pin);
    let mut ticker = Ticker::every(Duration::from_millis(4)); // 100 Hz
    let mut v = 0;
    loop {
        // ----- Acquire sensor data -----------------------------------------
        let angle = ANGLE_CH.receive().await;
        let strain = hx.read_raw().await;
        TARGET_CH.send((v as f32 / 100.0) * 2.0 * PI).await;
        v = (v + 1) % 100;
        // ----- Update LED ring ---------------------------------------------
        // Map 0-360° → 0-71 index.
        let idx =
            ((((angle / (2.0 * PI)) * LED_COUNT as f32 * 2.0) + 1.0) / 2.0) as usize % LED_COUNT;

        if strain > 400000 {
            leds.fill(RGB8 { r: 0, b: 5, g: 0 });
        } else {
            leds.fill(RGB8::default());
            leds[idx] = RGB8 {
                r: 255,
                g: 40,
                b: 0,
            };
        }
        //write_sk6805(&mut led_data, &leds);

        // ----- Logging ------------------------------------------------------

        ticker.next().await; // keep loop at 100 Hz
    }
}
