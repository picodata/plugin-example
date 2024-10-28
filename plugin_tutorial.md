# Создаем плагина для пикодаты

## Что такое плагины?
Плагины в Picodata позволяют делать всё, что угодно. Всё, что угодно, что является валидным Rust кодом, если говорить точнее. API плагинов в Picodata предоставляет фреймворк для создания распределенных приложений, которые будут работать в кластере СУБД.
Проще говоря, для того, чтобы создать свой плагин нужно реализовать лишь набор callback'ов, которые представлены трейтом Service.

Реализовав этот трейт мы получаем строительные кирпичики наших плагинов - сервисы. Их стоит рассматривать как классические web микросервисы из которых мы построим нашу систему.

## Что за плагин?
Мы попробуем создать плагин, который превратит Picodata из СУБД в настоящий throughput cache. Мы предоставим HTTP API, которое позволит запрос из OpenWeather текущую температуру по географическим координатам. При этом температуру по заданным координатам мы будем кешировать и сохранять в нашей СУБД и, если к нам придет еще один запрос с этими координатами, мы не будем совершать еще один запрос в OpenWeather, а отдадим закешированное значение. 
Для упрощения нашего примера мы не будем инвалидировать кеш.

## Как сделать плагин

### Сервис
Теперь пора приступать к созданию сервиса. Мы поддерживаем Rust как язык для создания плагинов, так что давайте начнем с инициализации крейта:
```bash
mkdir weather_cache && cd weather_cache
cargo init --lib
```

Обратите внимание на флаг `--lib` - мы будем собирать `*.so` или `*.dylib` библиотеку, а значит и крейт сразу инициализируем соответственно.
Давайте также сразу добавим строки для сборки `*.so` файла в `Cargo.toml`:
```
[lib]
crate-type = ["lib", "cdylib"]
```

А теперь добавим наш SDK в проект:
```bash
cargo add picodata_plugin
```

И с чистой совестью начнем реализовывать наш сервис в файле `src/lib.rs`:
```rust
use picodata_plugin::plugin::prelude::*;

struct WeatherService;

impl Service for WeatherService {
    type Config = ();
    fn on_config_change(
        &mut self,
        _ctx: &PicoContext,
        _new_cfg: Self::Config,
        _old_cfg: Self::Config,
    ) -> CallbackResult<()> {
        println!("I got a new config: {_new_cfg:?}");
        Ok(())
    }

    fn on_start(&mut self, _ctx: &PicoContext, _cfg: Self::Config) -> CallbackResult<()> {                
        println!("I started with config: {_cfg:?}");
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
    reg.add("weather_service", "0.1.0", WeatherService::new);
}
```

Давайте подробно взглянем на весь этот код.
Здесь происходит очень много всего, поэтому начнем с простого. У нас есть трейт `Service`, который необходимо реализовать. 

Первое на что мы обратим внимание - это тип `Config`. Он позволит нам описать конфигурацию нашего сервиса - например, адреса внешних узлов или значения таймаутов. Для нашего кэша мы пока не хотим ничего настраивать, поэтому определим тип пустым.

Перейдем к функциям сервиса, которые нам необходимо реализовать - начинает свою жизнь сервис с вызова `on_start`. Эта функция будет вызвана каждым узлом, где включается сервис или при вводе нового узла, где должен быть запущен сервис.
При [изменении конфигурации плагина](https://docs.picodata.io/picodata/devel/reference/sql/alter_plugin/) мы будет вызвана функция `on_config_change`, а старая и новые конфигурации будут переданы в качестве параметров.

О смене лидера в репликасете информирует вызов функции `on_leader_change`, что позволяет реагировать и запускать / останавливать фоновые операции.
И `gracefull shutdown` мы можем отработать в функции `on_stop`.

Пока что мы оставим тут заглушки и проверим, что наш минимальный плагин загружается и пишет что-то в лог.

### Манифест
Теперь нам необходимо как-то описать для Picodata как загрузить наш плагин. Сделаем мы это при помощи манифеста. Манифест - это файл, который описывает плагин, его составляющие и предоставляет Picodata необходимую для установки и запуска метаинформацию. Можно представить, что это такой аналог `cargo.toml` в пакетном менеджере `Cargo` или `package.json` в `npm`.

Создадим свой манифест с прозаичным наименованием `manifest.yaml`:
```yaml
# Имя плагина
name: weather_cache
# Описание плагина. Это метаданные, которые сейчас не используются Picdata, но могут администратору системы при установке или изучении установленных в кластер плагинов 
description: That one is created as an example of Picodata's plugin
# Версия плагина в формате semver. Picodata следит за установленными версиями плагинов и плагины с разными версиями - это разные объекты для Пикодаты
version: 0.1.0
# Список сервисов. Так как наш плагин не слишком сложный, нам хватит одного сервиса для реализации задуманного.
services:
    # Имя сервиса 
  - name: weather_service
    # Описание сервиса. Не используется внутри Picodata, но могут помочь администраторам системы
    description: This service provides HTTP route for a throughput weather cache
    # Конфигурация сервиса по умолчанию
    default_configuration:
```

### Пробный запуск
Соберем наш сервис:
```
cargo build
```

Теперь соберем в одно целое наш плагин и манифест и положим в одно место.
Здесь важно помнить, что для Picodata иерархия и структура директорий в [plugin-dir](https://docs.picodata.io/picodata/devel/reference/cli/#run_plugin_dir) имеет значение.
Picodata выполняет поиск плагина с учетом его имени и версии и ищется вот такой путь: `<plugin-dir>/<plugin-name>/<plugin-version>`. Для нашего плагина это будет выглядеть как `<plugin-dir>/weather_cache/0.1.0`

```bash 
mkdir -p build/weather_cache/0.1.0
cp target/debug/libweather_cache.so build/weather_cache/0.1.0
cp manifest.yaml build/weather_cache/0.1.0
``` 

У нас должна получится такая структура:
```
build
└── weather_cache
    └── 0.1.0
        ├── libweather_cache.so
        └── manifest.yaml
```

Теперь запустим Picodata и попробуем запустить плагин.

Запуск Picodata:
```bash
picodata run -l 127.0.0.1:3301 --advertise 127.0.0.1:3301 --peer 127.0.0.1:3301 --http-listen localhost:8081 --data-dir i1 --plugin-dir build
```

Запуск плагина:
```bash
$ picodata admin i1/admin.sock
Connected to admin console by socket path "i1/admin.sock"
type '\help' for interactive help
picodata> CREATE PLUGIN weather_cache 0.1.0
1
picodata> ALTER PLUGIN weather_cache 0.1.0 ADD SERVICE weather_service TO TIER default
1
picodata> ALTER PLUGIN weather_cache 0.1.0 ENABLE
1
```

После этого мы в логе Picodata увидим строку, которая свидетельствует, что наш плагин ожил и запустился.
```
I started with config: ()
```

Теперь давайте выключим и удалим плагин:
```
picodata> ALTER PLUGIN weather_cache 0.1.0 DISABLE
1
picodata> DROP PLUGIN weather_cache 0.1.0
1
```

Теперь давайте вспомним о следующей задаче. Наша цель - кэшировать где-то результаты запросов к внешнему сервису. Так как Picodata - это, в первую очередь, СУБД, нам стоит создать для этого таблицу. Система плагинов в Picodata позволяет создавать необходимые служебные таблицы для каждого плагина при помощи механизма миграций.
Для этого нам надо написать `SQL` команды, которые необходимо выполнить при установке плагина, а также те, которые необходимы для удаления этих таблиц (при удалении плагина).
У нас получится файл 0001_weather.sql:
```sql
-- pico.UP

CREATE TABLE "weather" (
    id UUID NOT NULL,
    latitude NUMBER NOT NULL,
    longitude NUMBER NOT NULL,
    temperature NUMBER NOT NULL,
    PRIMARY KEY (id)
)
USING memtx
DISTRIBUTED BY (latitude, longitude);

-- pico.DOWN
DROP TABLE "weather";
```

В этом файле есть специальные аннотации - `-- pico.UP` и `-- pico.DOWN`. Именно они помечают, какие команды выполнять на установке (`UP`) и удалении (`DOWN`).
Теперь нам необходимо положить его в папку с плагином и добавить его в наш манифест, чтобы Picodata могла знать, откуда его загрузить:
> Не забудьте также отредактировать манифест в `plugin_path`, а не только в репозитории

manifest.yaml
```yaml
name: weather_cache
description: That one is created as an example of Picodata's plugin
version: 0.1.0
services:
  - name: weather_service
    description: This service provides HTTP route for a throughput weather cache
    default_configuration:
      openweather_timeout: 5
migration:
  - 0001_weather.sql
```

А также добавим наш файл миграций в `plugin_path`:
```bash
cp 0001_weather.sql build/weather_cache/0.1.0
```

И еще раз установим плагин, но в этот раз мы еще запустим добавленные миграции:
```
$ picodata admin i1/admin.sock
Connected to admin console by socket path "i1/admin.sock"
type '\help' for interactive help
picodata> CREATE PLUGIN weather_cache 0.1.0
1
picodata> ALTER PLUGIN weather_cache 0.1.0 ADD SERVICE weather_service TO TIER default
1
picodata> ALTER PLUGIN weather_cache MIGRATE TO 0.1.0
1
picodata> ALTER PLUGIN weather_cache 0.1.0 ENABLE
1
```

Убедимся, что была создана таблица `weather`:
```
picodata> SELECT * FROM weather

+----+----------+-----------+-------------+
| id | latitude | longitude | temperature |
+=========================================+
+----+----------+-----------+-------------+
(0 rows)
```

Предлагаю снова очистить кластер, но на этот раз удалим также созданные миграциями таблицы:
```
picodata> ALTER PLUGIN weather_cache 0.1.0 DISABLE
1
picodata> DROP PLUGIN weather_cache 0.1.0 WITH DATA
1
```

А также убедимся в успешности `DOWN` миграции:
```
picodata> select * from weather
sbroad: table with name "weather" not found
```