use std::error::Error;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct WeatherInfo {
    city: String,
    temperature: i32,
}

pub(crate) fn weather_handler() -> Result<WeatherInfo, Box<dyn Error>> {
    Ok(WeatherInfo{
        city: "Empty".into(),
        temperature: 0,
    })
}

macro_rules! wrap_http_result {
    ($api_result:expr) => {{
        let mut status = 200;
        let mut content_type = "application/json";
        let content: String;
        match $api_result {
            Ok(res) => match serde_json::to_string(&res) {
                Ok(value) => content = value,
                Err(err) => {
                    content = err.to_string();
                    content_type = "plain/text";
                    status = 500
                }
            },
            Err(err) => {
                content = err.to_string();
                content_type = "plain/text";
                status = 500
            }
        }
        tlua::AsTable((
            ("status", status),
            ("body", content),
            ("headers", tlua::AsTable((("content-type", content_type),))),
        ))
    }};
}

pub(crate) use wrap_http_result;
