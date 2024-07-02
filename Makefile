debug-bin:
	cargo build --locked

release-bin:
	cargo build --locked --release

pack:
	rm -rf build && mkdir build
	cp -pr ./target/release/libplugin_test.so build/plugin_test.so
	cp -pr manifest.yaml build/
	cp -pr migrations build/

release: release-bin pack

build: debug-bin pack
