use anyhow::Ok;
use embedded_icon::iconoir::size24px::Position;
use embedded_icon::mdi::size24px::Et;
use epd_waveshare::buffer_len;
use epd_waveshare::color::TriColor;
use epd_waveshare::graphics::Display;
use epd_waveshare::{
    epd2in9b_v4::{Display2in9b, Epd2in9b},
    graphics::DisplayRotation,
    prelude::{WaveshareDisplay, WaveshareThreeColorDisplay},
};
use esp_idf_svc::hal::gpio::{AnyInputPin, AnyOutputPin};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::spi::SpiAnyPins;
use esp_idf_svc::hal::{
    delay::Ets,
    gpio as Gpio,
    gpio::{AnyIOPin, Input, Output, PinDriver},
    peripherals::Peripherals,
    spi::{
        config::{Config, DriverConfig},
        SpiDeviceDriver, SpiDriver,
    },
    units::FromValueType,
};

use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle, StyledDrawable},
    text::Text,
};

use embedded_graphics::image::Image;
use embedded_icon::{
    iconoir::size24px::{TemperatureHigh, WateringSoil},
    iconoir::size32px::{Cloud, Rain, SunLight, WarningCircle},
    NewIcon,
};

use crate::model::{Weather, WeatherForecast, WeatherReport};
pub struct EdpDisplay<'a> {
    spi: SpiDeviceDriver<'a, SpiDriver<'a>>,
    edp: Epd2in9b<
        SpiDeviceDriver<'a, SpiDriver<'a>>,
        PinDriver<'a, AnyInputPin, Input>,
        PinDriver<'a, AnyOutputPin, Output>,
        PinDriver<'a, AnyOutputPin, Output>,
        Ets,
    >,
    display: Display2in9b,
}

impl EdpDisplay<'_> {
    pub fn new<'a, SPI: SpiAnyPins>(
        spi: impl Peripheral<P = SPI> + 'a,
        sclk: &'a mut AnyOutputPin,
        sdo: &'a mut AnyOutputPin,
        cs: &'a mut AnyOutputPin,
        busy: &'a mut AnyInputPin,
        dc: &'a mut AnyOutputPin,
        rst: &'a mut AnyOutputPin,
    ) -> EdpDisplay<'a> {
        let bus_config = DriverConfig::new();
        let config = Config::new().baudrate(10u32.MHz().into());

        let mut spi = SpiDeviceDriver::new_single(
            spi,
            sclk,
            sdo,
            Option::<AnyIOPin>::None,
            Some(cs),
            &bus_config,
            &config,
        )
        .unwrap();

        let busy = PinDriver::input(busy).unwrap();
        let dc = PinDriver::output(dc).unwrap();
        let rst = PinDriver::output(rst).unwrap();
        let mut delay = Ets;

        let edp = Epd2in9b::new(&mut spi, busy, dc, rst, &mut delay, Some(50_000)).unwrap();
        let mut display = Display2in9b::default();

        display.set_rotation(DisplayRotation::Rotate270);
        _ = display.clear(TriColor::White);

        EdpDisplay { edp, spi, display }
    }

    pub fn display_weather(
        &mut self,
        weather_forcast: &Vec<WeatherForecast>,
        // current_weather: &WeatherReport,
    ) -> anyhow::Result<()> {
        let mut delay = Ets;

        // 296x128
        let text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_7X13_BOLD)
            .text_color(TriColor::Black)
            .build();

        let large_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
            .text_color(TriColor::Chromatic)
            .build();

        let week_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_7X13_BOLD)
            .text_color(TriColor::Chromatic)
            .build();

        let mut i = 0;

        for y in [53, 118] {
            for x in [0, 74, 148] {
                let fallback = WeatherForecast::default();
                let w = weather_forcast.get(i).unwrap_or(&fallback);

                let date_text = w.date.to_string();
                self.draw_icon(
                    w.weather,
                    Point {
                        x: x + 6,
                        y: y - 50,
                    },
                );
                let txt = format!("{}-{}C", w.min_temp, w.max_temp);
                let _ = Text::new(&txt, Point { x: x + 10, y }, text_style).draw(&mut self.display);
                let _ = Text::new(
                    &w.week,
                    Point {
                        x: x + 46,
                        y: y - 15,
                    },
                    week_text_style,
                )
                .draw(&mut self.display);
                let _ = Text::new(
                    &date_text,
                    Point {
                        x: x + 48,
                        y: y - 32,
                    },
                    large_text_style,
                )
                .draw(&mut self.display);
                i += 1;
            }
        }

        // Draw current temp on right
        /*
        self.draw_icon(current_weather.weather, Point { x: 250, y: 70 });

        let current_temp_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
            .text_color(TriColor::White)
            .build();

        let _ = Text::new(
            &current_weather.day.to_string(),
            Point { x: 232, y: 50 },
            current_temp_text_style,
        )
        .draw(&mut self.display);

        _ = Line::new(Point { x: 230, y: 60 }, Point { x: 280, y: 60 }).draw_styled(
            &PrimitiveStyle::with_stroke(TriColor::White, 3),
            &mut self.display,
        );

        let _ = Text::new(
            &format!("{}C", current_weather.temp).to_string(),
            Point { x: 250, y: 120 },
            current_temp_text_style,
        )
        .draw(&mut self.display);
        */
        self.edp.update_color_frame(
            &mut self.spi,
            &mut delay,
            self.display.bw_buffer(),
            self.display.chromatic_buffer(),
        )?;

        self.edp.display_frame(&mut self.spi, &mut delay)?;
        // self.edp.sleep(&mut self.spi, &mut delay)?;

        Ok(())
    }

    fn draw_base_frame(&mut self) {
        _ = self.display.fill_solid(
            &Rectangle::with_corners(Point { x: 225, y: 0 }, Point { x: 296, y: 128 }),
            TriColor::Black,
        );

        let black_link_style_thick = PrimitiveStyle::with_stroke(TriColor::Black, 2);
        let black_link_style = PrimitiveStyle::with_stroke(TriColor::Black, 1);

        _ = Line::new(Point { x: 225, y: 0 }, Point { x: 225, y: 128 })
            .draw_styled(&black_link_style_thick, &mut self.display);
        _ = Line::new(Point { x: 0, y: 63 }, Point { x: 225, y: 63 })
            .draw_styled(&black_link_style_thick, &mut self.display);

        _ = Line::new(Point { x: 74, y: 0 }, Point { x: 74, y: 128 })
            .draw_styled(&black_link_style, &mut self.display);
        _ = Line::new(Point { x: 148, y: 0 }, Point { x: 148, y: 128 })
            .draw_styled(&black_link_style, &mut self.display);
    }

    fn draw_icon(&mut self, weather: Weather, position: Point) {
        match weather {
            Weather::Sunny => {
                let _ = Image::new(&SunLight::new(TriColor::Chromatic), position)
                    .draw(&mut self.display);
            }
            Weather::Cloudly => {
                let _ =
                    Image::new(&Cloud::new(TriColor::Chromatic), position).draw(&mut self.display);
            }
            Weather::Rain => {
                let _ =
                    Image::new(&Rain::new(TriColor::Chromatic), position).draw(&mut self.display);
            }
            Weather::Unknow => {
                let _ = Image::new(&WarningCircle::new(TriColor::Chromatic), position)
                    .draw(&mut self.display);
            }
        };
    }

    pub fn display_current_temperature(
        &mut self,
        temperature: f32,
        humidity: f32,
        partial_update: bool,
    ) {
        let delay = &mut Ets;

        if !partial_update {
            self.draw_base_frame();

            let _ = Image::new(
                &TemperatureHigh::new(TriColor::Chromatic),
                Point { x: 228, y: 12 },
            )
            .draw(&mut self.display);

            let _ = Image::new(
                &WateringSoil::new(TriColor::Chromatic),
                Point { x: 228, y: 64 },
            )
            .draw(&mut self.display);

            let _ = self.edp.update_color_frame(
                &mut self.spi,
                delay,
                &mut self.display.bw_buffer(),
                &mut self.display.chromatic_buffer(),
            );

            _ = self.edp.update_and_display_frame_base(
                &mut self.spi,
                self.display.bw_buffer(),
                Some(self.display.chromatic_buffer()),
                delay,
            );

            // let _ = self.edp.display_frame(&mut self.spi, delay);
        } else {
            // Will rotate the screen for drawing, so width is the shorter side
            const TEXT_SIZE: Size = Size {
                width: 24,
                height: 40,
            };

            let text_style = MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_7X13_BOLD)
                .text_color(TriColor::White)
                .build();

            let temp_text = format!("{:.1}C", temperature);
            let humidity_text = format!("{:.1}%", humidity);

            log::info!("temp: {temp_text}");

            let mut text_display = epd_waveshare::graphics::Display::<
                { TEXT_SIZE.width },
                { TEXT_SIZE.height },
                false,
                { buffer_len(TEXT_SIZE.height as usize, TEXT_SIZE.width as usize * 2) },
                TriColor,
            >::default();

            text_display.set_rotation(DisplayRotation::Rotate270);

            _ = text_display.clear(TriColor::Black);
            _ = Text::new(temp_text.as_str(), Point { x: 0, y: 16 as i32 }, text_style)
                .draw(&mut text_display);

            _ = self.edp.update_partial_frame(
                &mut self.spi,
                delay,
                text_display.bw_buffer(),
                22,
                0,
                TEXT_SIZE.width,
                TEXT_SIZE.height,
            );

            _ = text_display.clear(TriColor::Black);
            _ = Text::new(
                &humidity_text.as_str(),
                Point { x: 0, y: 16 as i32 },
                text_style,
            )
            .draw(&mut text_display);

            _ = self.edp.update_partial_frame(
                &mut self.spi,
                delay,
                text_display.bw_buffer(),
                70,
                0,
                TEXT_SIZE.width,
                TEXT_SIZE.height,
            );
            _ = self.edp.display_frame_partial(&mut self.spi, delay);
        }
    }

    pub fn sleep(&mut self) {
        _ = self.edp.sleep(&mut self.spi, &mut Ets);
    }

    pub fn wake_up(&mut self) {
        _ = self.edp.wake_up(&mut self.spi, &mut Ets);
    }
}
