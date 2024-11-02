# Weather Cache

Throughput cache for open-meteo.

## How to build
```
cargo build
```

## How to run
Use [cargo-pike](https://git.picodata.io/picodata/plugin/cargo-pike)
```
cargo pike run --topology topology.toml --data-dir ./tmp
```

## Config
### TTL
Specifies a number of seconds before record is expired and deleted

### Timeout
Specifies a number of second for HTTP call to open-meteo