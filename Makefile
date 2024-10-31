debug-bin:
	cargo build --locked

release-bin:
	cargo build --locked --release

pack:
	rm -rf build && mkdir -p build/weather_cache/0.2.0
	cp -pr manifest.yaml build/weather_cache/0.2.0/
	cp -pr migrations build/weather_cache/0.2.0/

release-pack: pack
	cp -pr ./target/release/libplugin_test.so build/weather_cache/0.2.0/weather_cache.so

debug-pack: pack
	cp -pr ./target/debug/libplugin_test.so build/weather_cache/0.2.0/weather_cache.so

release: release-bin release-pack

build: debug-bin debug-pack
