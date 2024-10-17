use std::{error::Error, time::Duration};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct CurrentWeather {
    temperature_2m: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct StoredWeatherInfo {
    id: picodata_plugin::system::tarantool::uuid::Uuid,
    latitude: f64,
    longitude: f64,
    temperature: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct WeatherInfo {
    latitude: f64,
    longitude: f64,
    current: CurrentWeather,
}

static METEO_URL: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
    std::env::var("METEO_URL").unwrap_or(String::from("https://api.open-meteo.com"))
});

fn weather_request(
    latitude: f64,
    longitude: f64,
    request_timeout: u64,
) -> Result<WeatherInfo, Box<dyn Error>> {
    let http_client = fibreq::ClientBuilder::new().build();
    let http_req = http_client
        .get(format!(
            "{url}/v1/forecast?\
            latitude={latitude}&\
            longitude={longitude}&\
            current=temperature_2m",
            url = METEO_URL.as_str(),
            latitude = latitude,
            longitude = longitude
        ))
        .unwrap();

    let mut http_resp = http_req
        .request_timeout(Duration::from_secs(request_timeout))
        .send()
        .unwrap();

    let resp_body = http_resp.text().unwrap();
    let info = serde_json::from_str(&resp_body).unwrap();

    Ok(info)
}

pub(crate) fn weather_handler(
    latitude: f64,
    longitude: f64,
) -> Result<WeatherInfo, Box<dyn Error>> {
    let select_query: &str = r#"
    select * from "weather" 
    where 
        (latitude < (? + 0.5) AND latitude > (? - 0.5))
        AND
        (longitude < (? + 0.5) AND longitude > (? - 0.5));
    "#;
    let res = picodata_plugin::sql::query(select_query)
        .bind(latitude)
        .bind(latitude)
        .bind(longitude)
        .bind(longitude)
        .fetch::<StoredWeatherInfo>()
        .unwrap();

    if !res.is_empty() {
        let weather_info = WeatherInfo {
            latitude: res[0].latitude,
            longitude: res[0].longitude,
            current: CurrentWeather {
                temperature_2m: res[0].temperature,
            },
        };
        return Ok(weather_info);
    }
    let request_timeout = 5;

    let res = match weather_request(latitude, longitude, request_timeout) {
        Ok(weather_info) => weather_info,
        Err(e) => todo!("failed to run request %{e}"),
    };

    let insert_query: &str = r#"
        INSERT INTO "weather"
        VALUES(?, ?, ?, ?)
    "#;
    let uuid = picodata_plugin::system::tarantool::uuid::Uuid::random();
    let _ = picodata_plugin::sql::query(insert_query)
        .bind(uuid)
        .bind(res.latitude)
        .bind(res.longitude)
        .bind(res.current.temperature_2m)
        .execute()
        .unwrap();

    Ok(res)
}

pub(crate) fn hello_handler() -> Result<String, Box<dyn Error>> {
    Ok("Hello, world".into())
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
