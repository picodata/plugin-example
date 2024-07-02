use std::collections::HashMap;

use picoplugin::plugin::interface::{CallbackResult, DDL};
use picoplugin::plugin::prelude::*;
use picoplugin::system::tarantool::tlua;
use serde::{Deserialize, Serialize};
use linkme;

mod http;

#[derive(Serialize, Deserialize, Debug, tlua::Push)]
struct WeatherServiceCfg {
    openweather_timeout: i32,
}

struct WeatherService {
    cfg: Option<WeatherServiceCfg>,
}

impl Service for WeatherService {
    type CFG = WeatherServiceCfg;

    fn on_cfg_validate(&self, _configuration: Self::CFG) -> CallbackResult<()> {

        Ok(())
    }

    fn on_config_change(
        &mut self,
        ctx: &PicoContext,
        new_cfg: Self::CFG,
        _old_cfg: Self::CFG,
    ) -> CallbackResult<()> {

        Ok(())
    }

    fn schema(&self) -> Vec<DDL> {
        vec![]
    }

    fn on_start(&mut self, ctx: &PicoContext, cfg: Self::CFG) -> CallbackResult<()> {
        let lua = picoplugin::system::tarantool::lua_state();
        lua.exec_with(
            "pico.httpd:route({method = 'GET', path = '/hello' }, ...)",
            tlua::Function::new(|| -> _ {
                http::wrap_http_result!(http::hello_handler())
            }),
        ).unwrap();
        
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
        ).unwrap();
                
        Ok(())
    }

    fn on_stop(&mut self, ctx: &PicoContext) -> CallbackResult<()> {
        let lua = picoplugin::system::tarantool::lua_state();
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
        "#
        ).unwrap();
        Ok(())
    }

    /// Called after replicaset master is changed
    fn on_leader_change(&mut self, ctx: &PicoContext) -> CallbackResult<()> {
        Ok(())
    }
}

impl WeatherService {
    pub fn new() -> Self {
        WeatherService { cfg: None }
    }
}

#[service_registrar]
pub fn service_registrar(reg: &mut ServiceRegistry) {
    reg.add("weather_service", "0.1.0", WeatherService::new);
}
