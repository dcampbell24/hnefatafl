.PHONY: coverage enable-git-hooks profile-games profile-monte-carlo logs ssl cargo-deps js html-client android macos

coverage:
	CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='./.coverage/cargo-test-%p-%m.profraw' cargo test
	grcov ./.coverage --binary-path ./target/debug/deps/ \
	                  -s . \
					  -t html \
					  --branch \
					  --ignore-not-existing \
					  --ignore '../*' \
					  --ignore "/*" \
					  -o ./coverage/html

enable-git-hooks:
	git config --local core.hooksPath .githooks/

profile-games:
	echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
	samply record cargo test hnefatafl_games --no-default-features --profile profiling

profile-monte-carlo:
	echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
	samply record cargo test monte_carlo_long --no-default-features --profile profiling -- --ignored

logs:
	sudo journalctl --unit=hnefatafl.service --reverse

ssl:
	openssl \
	req -x509 \
	-newkey rsa:4096 \
	-keyout ssl/localhost.key \
	-out ssl/localhost.crt \
	-sha256 \
	-days 3650 \
	-nodes \
	-subj '/CN=localhost'
	
	sudo cp ssl/localhost.crt /usr/local/share/ca-certificates/
	sudo update-ca-certificates

cargo-deps:
	python3 ../flatpak-builder-tools/cargo/flatpak-cargo-generator.py Cargo.lock -o packages/flathub/cargo-sources.json

js:
	wasm-pack build --target web --no-default-features --features js

html-client:
	sudo cp html-client/index.html /var/www/html/index.html
	sudo mkdir --parent /var/www/html/pkg
	sudo cp -r pkg /var/www/html

android:
	set ANDROID_NDK ${HOME}/Android/Sdk/ndk \
	cargo build --example hnefatafl-client --target aarch64-linux-android --no-default-features

macos:
	cargo bundle --bin hnefatafl-client --features client --no-default-features --release
