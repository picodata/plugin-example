use std::{error::Error, time::Duration};
use serde::Deserialize;
use serde::Serialize;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurrentWeather {
    pub temperature_2m: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct WeatherInfo {
    pub latitude: f64,
    pub longitude: f64,
    pub current: CurrentWeather,
}

static METEO_URL: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
    std::env::var("METEO_URL").unwrap_or(String::from("https://api.open-meteo.com"))
});

pub fn weather_request(latitude: f64, longitude: f64, request_timeout: u64) -> Result<WeatherInfo, Box<dyn Error>> {
    let http_client  = fibreq::ClientBuilder::new().build();
    let http_req = http_client
        .get(format!(
            "{url}/v1/forecast?\
            latitude={latitude}&\
            longitude={longitude}&\
            current=temperature_2m",
            url = METEO_URL.as_str(),
            latitude = latitude,
            longitude = longitude
        ))?;
        
    let mut http_resp = http_req.request_timeout(Duration::from_secs(request_timeout)).send()?;
    
    let resp_body = http_resp.text()?;
    let info = serde_json::from_str(&resp_body)?;

    return Ok(info)
}
