use picoplugin::plugin::interface::{CallbackResult, ErrorBox, DDL};
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
            "pico.httpd:route({method = 'GET', path = 'api/v1/weather' }, ...)",
            tlua::Function::new(|| -> _ {
                http::wrap_http_result!(http::weather_handler)
            }),
        ).unwrap();
        Ok(())
    }

    fn on_stop(&mut self, ctx: &PicoContext) -> CallbackResult<()> {
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
    reg.add("ServiceExample", "0.1.0", WeatherService::new);
}
