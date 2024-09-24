debug-bin:
	cargo build --locked

release-bin:
	cargo build --locked --release

pack:
	rm -rf build && mkdir build
	cp -pr ./target/debug/libplugin_test.so build/weather_cache.so
	cp -pr manifest.yaml build/
	cp -pr migrations build/

release: release-bin pack

build: debug-bin pack
