use core::str;
use embedded_svc::http::client::Client;
use esp_idf_svc::http::client::EspHttpConnection;
use serde_json::Value;
use std::fmt;

use crate::model::*;

pub trait HttpClient {
    fn get_request(&mut self, url: &str) -> Result<String, ApiError>;
}

const WEATHER_API_URL: &str =
    "https://data.weather.gov.hk/weatherAPI/opendata/weather.php?dataType=fnd&lang=en";

const WEATHER_REPORT_API_URL: &str =
    "https://data.weather.gov.hk/weatherAPI/opendata/weather.php?dataType=rhrread&lang=en";

pub struct WeatherApi<C: HttpClient> {
    http_client: C,
}

impl Weather {
    pub fn from_icon_code(code: u8) -> Self {
        match code {
            50 | 51 | 52 => Self::Sunny,
            53 | 60 | 61 | 76 | 77 => Self::Cloudly,
            54 | 62 | 63 | 64 | 65 => Self::Rain,
            _ => Self::Unknow,
        }
    }
}

impl<C> WeatherApi<C>
where
    C: HttpClient,
{
    pub fn new(client: C) -> Self {
        Self {
            http_client: client,
        }
    }

    fn get_request_json(&mut self, url: &str) -> Result<Value, ApiError> {
        let response = self.http_client.get_request(url)?;
        let json = match serde_json::from_str(response.as_str()) as Result<Value, serde_json::Error>
        {
            Ok(json) => json.to_owned(),
            Err(_err) => return Err(ApiError::JsonError),
        };

        Ok(json)
    }

    pub fn fetch_current_weather(&mut self) -> Result<WeatherReport, ApiError> {
        let json = self.get_request_json(WEATHER_REPORT_API_URL)?;
        let default = Vec::new();
        let array = json["temperature"]["data"].as_array().unwrap_or(&default);

        let icon_code = json["icon"][0].as_u64().unwrap_or(60) as u8;
        let weather = Weather::from_icon_code(icon_code);
        let date_full_string = json["updateTime"].as_str();
        log::info!("{:?}", date_full_string);
        let day = date_full_string.map(|ds| &(ds[5..10])).unwrap_or("--");

        let weather_region: Vec<WeatherReport> = array
            .iter()
            .map(|w| {
                let place = w["place"].as_str().unwrap_or_default();
                let temperature = w["value"].as_i64();

                WeatherReport {
                    place: place.to_owned(),
                    temp: temperature.unwrap_or(0) as i8,
                    weather,
                    day: day.to_owned(),
                }
            })
            .collect();

        if weather_region.is_empty() {
            return Err(ApiError::ResponseError);
        }

        match weather_region.iter().find(|w| w.place == "Sham Shui Po") {
            Some(val) => Ok(val.to_owned()),
            None => Ok(weather_region[0].clone()),
        }
    }

    pub fn fetch_local_weather_forecast(&mut self) -> Result<Vec<WeatherForecast>, ApiError> {
        let json = self.get_request_json(WEATHER_API_URL);

        let weather_forcase_json_array = match json {
            Ok(json) => json["weatherForecast"].as_array().unwrap().to_owned(),
            Err(_err) => return Err(ApiError::JsonError),
        };

        let data: Vec<WeatherForecast> = weather_forcase_json_array
            .iter()
            .map(|w| {
                let min_temp = w["forecastMintemp"]["value"].as_i64().unwrap_or_default() as i8;
                let max_temp = w["forecastMaxtemp"]["value"].as_i64().unwrap_or_default() as i8;
                let date = w["forecastDate"].as_str().unwrap_or_default();
                let week = w["week"].as_str().unwrap_or_default().to_uppercase();

                let day = if date.len() > 2 {
                    date[date.len() - 2..date.len()].to_owned()
                } else {
                    "".to_owned()
                };

                let week = if week.len() > 3 {
                    week[..3].to_owned()
                } else {
                    "".to_owned()
                };

                let day_int = day.parse().unwrap_or_default();
                let forecast_icon_code = w["ForecastIcon"].as_i64().unwrap_or_default() as u8;

                WeatherForecast {
                    max_temp,
                    min_temp,
                    week,
                    date: day_int,
                    weather: Weather::from_icon_code(forecast_icon_code),
                }
            })
            .collect();

        return Ok(data);
    }
}

impl HttpClient for Client<EspHttpConnection> {
    fn get_request(&mut self, url: &str) -> Result<String, ApiError> {
        let req = self.get(url).unwrap();
        let mut res = req.submit().unwrap();

        let mut buffer = [0_u8; 256];
        let mut result: Vec<u8> = Vec::new();

        // TODO: Status code checking
        // if res.status()

        while let Ok(size) = res.read(&mut buffer) {
            if size == 0 {
                break;
            }

            result.extend_from_slice(&buffer[..size]);
        }

        let string = str::from_utf8(&result).map_err(|_err| ApiError::ResponseError)?;

        Ok(string.to_owned())
    }
}
