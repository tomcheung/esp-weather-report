mod edp_display;
mod http_client;
mod model;
mod weather_api;
mod wifi_config;

use dht_embedded::{Dht22, DhtSensor, NoopInterruptControl};
use edp_display::EdpDisplay;

use esp_idf_svc::hal::{
    delay::Delay,
    gpio::{AnyInputPin, AnyOutputPin, PinDriver},
    modem::Modem,
};
use http_client::{get_http_client, setup_wifi};

use anyhow::{Ok, Result};
use esp_idf_svc::hal::peripherals::Peripherals;

use weather_api::WeatherApi;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::set_max_level(log::LevelFilter::Debug);
    let delay = Delay::default();

    let peripheral = Peripherals::take().unwrap();

    let senser_pin = peripheral.pins.gpio21;

    let senser_pin_driver = PinDriver::input_output(senser_pin).unwrap();
    let mut sensor = Dht22::new(NoopInterruptControl, delay, senser_pin_driver);

    let mut sclk: AnyOutputPin = peripheral.pins.gpio13.into();
    let mut sdo: AnyOutputPin = peripheral.pins.gpio14.into();
    let mut cs: AnyOutputPin = peripheral.pins.gpio15.into();
    let mut busy: AnyInputPin = peripheral.pins.gpio25.into();
    let mut dc: AnyOutputPin = peripheral.pins.gpio27.into();
    let mut rst: AnyOutputPin = peripheral.pins.gpio26.into();

    let mut display = EdpDisplay::new(
        peripheral.spi3,
        &mut sclk,
        &mut sdo,
        &mut cs,
        &mut busy,
        &mut dc,
        &mut rst,
    );

    let mut modem = peripheral.modem;
    dispaly_weather(&mut modem, &mut display)?;

    let mut count: u16 = 0;
    loop {
        match sensor.read() {
            std::result::Result::Ok(reading) => {
                display.display_current_temperature(
                    reading.temperature(),
                    reading.humidity(),
                    count > 0,
                );

                count += 1;

                delay.delay_ms(10000);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                delay.delay_ms(1200);
            }
        }

        if count == 60 {
            display.sleep();
            log::info!("sleep");
            delay.delay_ms(12000);
            count = 0;

            log::info!("reset display");
            display.wake_up();
        }
    }
}

fn dispaly_weather(modem: &mut Modem, display: &mut EdpDisplay) -> Result<()> {
    let wifi = setup_wifi(modem)?;
    let client = get_http_client();
    let mut api = WeatherApi::new(client);
    let forcase = api.fetch_local_weather_forecast()?;
    // let current_weather = api.fetch_current_weather()?;

    display.display_weather(&forcase)?;
    // Result<(Vec<WeatherForecast>, WeatherReport)>

    Ok(())
}
