use picodata_plugin::background::CancellationToken;
use serde::Deserialize;
use serde::Serialize;

use once_cell::unsync::Lazy;
use picodata_plugin::plugin::prelude::*;
use picodata_plugin::system::tarantool::{clock::time, say_info};
use shors::transport::http::route::Builder;
use shors::transport::http::{server, Request, route::Handler, Response};
use shors::transport::Context;
use tarantool::say_error;

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

const TTL_JOB_NAME: &str = "ttl-worker";

const SELECT_QUERY: &str = r#"
SELECT * FROM weather 
WHERE
    latitude = ?
    AND
    longitude = ?;
"#;

const INSERT_QUERY: &str = r#"
INSERT INTO "weather"
VALUES(?, ?, ?, ?)
"#;

const TTL_QUERY: &str = r#"
    DELETE FROM weather WHERE (latitude, longitude) IN (
        SELECT latitude, longitude FROM weather
            WHERE created_at <= ?
            LIMIT 10
    );
"#;

struct WeatherService;

#[derive(Serialize, Deserialize, Debug)]
struct ServiceCfg {
    timeout: u64,
    ttl: i64
}

fn error_handler_middleware(handler: Handler<Box<dyn Error>>) -> Handler<Box<dyn Error>> {
    Handler(Box::new(move |ctx, request| {
        let inner_res = handler(ctx, request);
        let resp = match inner_res {
            Ok(resp) => resp,
            Err(err) => {
                say_error!("{err:?}");
                let mut resp: Response = Response::from(err.to_string());
                resp.status = 500;
                resp
            }
        };

        return Ok(resp);
}))
}

fn get_ttl_job(ttl: i64) -> impl Fn(CancellationToken) {
    move |ct: CancellationToken| {
            while ct.wait_timeout(Duration::from_secs(1)).is_err() {
                let expired = time() as i64 - ttl;
                match picodata_plugin::sql::query(&TTL_QUERY)
                .bind(expired)
                .execute() {
                    Ok(rows_affected) => {
                        say_info!("Cleaned {rows_affected:?} expired records");
                    },
                    Err(error) => {
                        say_error!("Error while cleaning expired records: {error:?}")
                    }
                };

            }
        say_info!("TTL worker stopped");
    }
}

impl Service for WeatherService {
    type Config = ServiceCfg;
    
    fn on_config_change(
        &mut self,
        ctx: &PicoContext,
        new_cfg: Self::Config,
        _old_cfg: Self::Config,
    ) -> CallbackResult<()> {
        TIMEOUT.set(Duration::from_secs(new_cfg.timeout));
        let wm = ctx.worker_manager();
        wm.cancel_tagged(TTL_JOB_NAME, Duration::from_secs(1))?;
        wm.register_tagged_job(get_ttl_job(new_cfg.ttl), TTL_JOB_NAME)?;
        Ok(())
    }

    fn on_start(&mut self, ctx: &PicoContext, cfg: Self::Config) -> CallbackResult<()> {
        say_info!("I started with config: {cfg:?}");

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
            created_at: i64,
        }

        let weather_endpoint = Builder::new()
            .with_method("POST")
            .with_path("/weather")
            .with_middleware(error_handler_middleware)
            .build(
                |_ctx: &mut Context, request: Request| -> Result<_, Box<dyn Error>> {
                    let req: WeatherReq = request.parse()?;
                    let latitude = req.latitude;
                    let longitude = req.longitude;

                    // we shall store latitude / longitude as integer
                    // to avoid comparing floating point numbers
                    let cached: Vec<Weather> = picodata_plugin::sql::query(&SELECT_QUERY)
                        .bind(latitude)
                        .bind(longitude)
                        .fetch::<Weather>()
                        .map_err(|err| {
                            format!("failed to retrieve data: {err}")
                        })?;
                    if !cached.is_empty() {
                        let resp = cached[0].clone();
                        return Ok(resp);
                    }
                    let openweather_resp =
                        openweather::weather_request(req.latitude, req.longitude, 3)?;
                    // use request coordinates because openweather
                    // select the nearest avaliable coordinates
                    // that may not match the exact request coords
                    let resp: Weather = Weather {
                        latitude: latitude,
                        longitude: longitude,
                        temperature: openweather_resp.current.temperature_2m,
                        created_at: time() as i64,
                    };

                    let _ = picodata_plugin::sql::query(&INSERT_QUERY)
                        .bind(latitude)
                        .bind(longitude)
                        .bind(resp.temperature)
                        .bind(resp.created_at)
                        .execute()
                        .map_err(|err| format!("failed to retrieve data: {err}"))?;

                    Ok(resp)
                },
            );
        
        HTTP_SERVER.with(|srv| {
            srv.register(Box::new(hello_endpoint));
            srv.register(Box::new(weather_endpoint));
        });

        let wm = ctx.worker_manager();
        let ttl_job = get_ttl_job(cfg.ttl);
        wm.register_tagged_job(ttl_job, TTL_JOB_NAME).unwrap();

        Ok(())
    }

    fn on_stop(&mut self, ctx: &PicoContext) -> CallbackResult<()> {
        say_info!("I stopped with config");

        let wm = ctx.worker_manager();
        wm.cancel_tagged(TTL_JOB_NAME, Duration::from_secs(1))?;

        Ok(())
    }

    /// Called after replicaset master is changed
    fn on_leader_change(&mut self, _ctx: &PicoContext) -> CallbackResult<()> {
        say_info!("Leader has changed!");
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
    reg.add("weather_service", env!("CARGO_PKG_VERSION"), WeatherService::new);
}
