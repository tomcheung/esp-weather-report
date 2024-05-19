use std::{fmt, str::Utf8Error};

#[derive(Debug)]
pub enum ApiError {
    ResponseError,
    ParseError(Utf8Error),
    JsonError,
}

#[derive(Clone, Copy)]
pub enum Weather {
    Sunny,
    Cloudly,
    Rain,
    Unknow,
}
pub struct WeatherForecast {
    pub date: u8,
    pub week: String,
    pub max_temp: i8,
    pub min_temp: i8,
    pub weather: Weather,
}

#[derive(Clone)]
pub struct WeatherReport {
    pub place: String,
    pub temp: i8,
    pub weather: Weather,
    pub day: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiError {}", self)
    }
}

impl std::error::Error for ApiError {}

impl Default for WeatherForecast {
    fn default() -> Self {
        Self {
            min_temp: 0,
            max_temp: 0,
            date: 0,
            week: String::from("---"),
            weather: Weather::Unknow,
        }
    }
}
