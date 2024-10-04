debug-bin:
	cargo build --locked

release-bin:
	rm -rf ./target/debug
	cargo build --locked --release

pack:
	rm -rf build && mkdir build
	cp -pr manifest.yaml build/
	cp -pr migrations build/

release-pack: pack
	cp -pr ./target/release/libplugin_test.so build/weather_cache.so

debug-pack: pack
	cp -pr ./target/debug/libplugin_test.so build/weather_cache.so

release: release-bin release-pack

build: debug-bin debug-pack
