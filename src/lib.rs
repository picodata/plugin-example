use std::collections::HashMap;

use picodata_plugin::plugin::interface::CallbackResult;
use picodata_plugin::plugin::prelude::*;
use picodata_plugin::system::tarantool::tlua;
use serde::{Deserialize, Serialize};

mod http;

#[derive(Serialize, Deserialize, Debug, tlua::Push)]
struct WeatherServiceCfg {
    openweather_timeout: i32,
}

struct WeatherService;

impl Service for WeatherService {
    type Config = WeatherServiceCfg;
    fn on_config_change(
        &mut self,
        _ctx: &PicoContext,
        _new_cfg: Self::Config,
        _old_cfg: Self::Config,
    ) -> CallbackResult<()> {
        Ok(())
    }

    fn on_start(&mut self, _ctx: &PicoContext, _cfg: Self::Config) -> CallbackResult<()> {
        let lua = picodata_plugin::system::tarantool::lua_state();
        lua.exec_with(
            "pico.httpd:route({method = 'GET', path = '/hello' }, ...)",
            tlua::Function::new(|| -> _ { http::wrap_http_result!(http::hello_handler()) }),
        )
        .unwrap();

        lua.exec_with(
            "
            local function make_json_handler(fn)
                return function(req)
                    return fn(req.query)
                end
            end

            pico.httpd:route(
                { method = 'GET', path = 'api/v1/weather' },
                    make_json_handler(...)
            )
            ",
            tlua::function1(move |query: String| -> _ {
                let params: HashMap<String, f64> = serde_qs::from_str(&query).unwrap();
                let longitude = params.get("longitude").unwrap();
                let latitude = params.get("latitude").unwrap();
                http::wrap_http_result!(http::weather_handler(*longitude, *latitude))
            }),
        )
        .unwrap();

        Ok(())
    }

    fn on_stop(&mut self, _ctx: &PicoContext) -> CallbackResult<()> {
        let lua = picodata_plugin::system::tarantool::lua_state();
        lua.exec(
            r#"
            local httpd = pico.httpd
            if httpd ~= nil then
                for n = 1, table.maxn(httpd.routes) do
                    local r = httpd.routes[n]
                    if r == nil then
                        goto continue
                    end
                    if not r.name or not r.name:startswith("kirovets-api") then
                        goto continue
                    end
        
                    log.info("Removing kirovets HTTP route %q (%s)", r.path, r.method)
                    if httpd.iroutes[r.name] ~= nil then
                        httpd.iroutes[r.name] = nil
                    end
                    httpd.routes[n] = nil
        
                    ::continue::
                end
            end
        "#,
        )
        .unwrap();
        Ok(())
    }

    /// Called after replicaset master is changed
    fn on_leader_change(&mut self, _ctx: &PicoContext) -> CallbackResult<()> {
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
    reg.add("weather_service", "0.1.0", WeatherService::new);
}
