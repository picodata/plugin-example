use serde::Deserialize;
use serde::Serialize;

use once_cell::unsync::Lazy;
use picodata_plugin::plugin::prelude::*;
use shors::transport::http::route::Builder;
use shors::transport::http::{server, Request};
use shors::transport::Context;

use std::cell::Cell;
use std::error::Error;
use std::time::Duration;

mod openweather;

thread_local! {
    pub static HTTP_SERVER: Lazy<server::Server> = Lazy::new(server::Server::new);
}
thread_local! {
    pub static TIMEOUT: Cell<Duration> = Cell::new(Duration::from_secs(3));
}

const SELECT_QUERY: &str = r#"
select * from "weather" 
where 
    (latitude < (? + 0.5) AND latitude > (? - 0.5))
    AND
    (longitude < (? + 0.5) AND longitude > (? - 0.5));
"#;

const INSERT_QUERY: &str = r#"
INSERT INTO "weather"
VALUES(?, ?, ?)
"#;

struct WeatherService;

#[derive(Serialize, Deserialize, Debug)]
struct ServiceCfg {
    timeout: u64
}

impl Service for WeatherService {
    type Config = ServiceCfg;
    fn on_config_change(
        &mut self,
        _ctx: &PicoContext,
        new_cfg: Self::Config,
        _old_cfg: Self::Config,
    ) -> CallbackResult<()> {
        TIMEOUT.set(Duration::from_secs(new_cfg.timeout));
        Ok(())
    }

    fn on_start(&mut self, _ctx: &PicoContext, _cfg: Self::Config) -> CallbackResult<()> {
        println!("I started with config: {_cfg:?}");

        let hello_endpoint = Builder::new().with_method("GET").with_path("/hello").build(
            |_ctx: &mut Context, _: Request| -> Result<_, Box<dyn Error>> {
                Ok("Hello, World!".to_string())
            },
        );

        #[derive(Serialize, Deserialize)]
        pub struct WeatherReq {
            latitude: f64,
            longitude: f64,
        }

        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct Weather {
            latitude: f64,
            longitude: f64,
            temperature: f64,
        }

        let weather_endpoint = Builder::new()
            .with_method("POST")
            .with_path("/weather")
            .build(
                |_ctx: &mut Context, request: Request| -> Result<_, Box<dyn Error>> {
                    let req: WeatherReq = request.parse()?;
                    let latitude = req.latitude;
                    let longitude = req.longitude;

                    let cached: Vec<Weather> = picodata_plugin::sql::query(&SELECT_QUERY)
                        .bind(latitude)
                        .bind(latitude)
                        .bind(longitude)
                        .bind(longitude)
                        .fetch::<Weather>()
                        .map_err(|err| format!("failed to retrieve data: {err}"))?;
                    if !cached.is_empty() {
                        let resp = cached[0].clone();
                        return Ok(resp);
                    }
                    let openweather_resp =
                        openweather::weather_request(req.latitude, req.longitude, 3)?;
                    let resp: Weather = Weather {
                        latitude: openweather_resp.latitude,
                        longitude: openweather_resp.longitude,
                        temperature: openweather_resp.current.temperature_2m,
                    };

                    let _ = picodata_plugin::sql::query(&INSERT_QUERY)
                        .bind(resp.latitude)
                        .bind(resp.longitude)
                        .bind(resp.temperature)
                        .execute()
                        .map_err(|err| format!("failed to retrieve data: {err}"))?;

                    Ok(resp)
                },
            );

        HTTP_SERVER.with(|srv| {
            srv.register(Box::new(hello_endpoint));
            srv.register(Box::new(weather_endpoint));
        });

        Ok(())
    }

    fn on_stop(&mut self, _ctx: &PicoContext) -> CallbackResult<()> {
        println!("I stopped with config");

        Ok(())
    }

    /// Called after replicaset master is changed
    fn on_leader_change(&mut self, _ctx: &PicoContext) -> CallbackResult<()> {
        println!("Leader has changed!");
        Ok(())
    }
}

impl WeatherService {
    pub fn new() -> Self {
        WeatherService {}
    }
}

#[service_registrar]
pub fn service_registrar(reg: &mut ServiceRegistry) {
    reg.add("weather_service", "0.2.0", WeatherService::new);
}
