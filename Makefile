.PHONY: release

dev:
	virtualenv -p python3 venv
	. venv/bin/activate && pip install -r poke-engine-py/requirements.txt && pip install -r poke-engine-py/requirements-dev.txt && cd poke-engine-py && maturin develop --features="poke-engine/terastallization"

upload_python_bindings:
	cd poke-engine-py && ./build_and_publish

upload_rust_lib:
	cargo publish --features "terastallization"

release:
	./release

fmt:
	cargo fmt
	ruff format poke-engine-py

gen9:
	cargo build --release --features gen9,terastallization --no-default-features

test:
	cargo test --no-default-features --features "terastallization"
	cargo test --no-default-features --features "gen9"

install_ci:
	pip install -r poke-engine-py/requirements.txt
	pip install -r poke-engine-py/requirements-dev.txt
	cd poke-engine-py && maturin develop --features="poke-engine/terastallization"

fmt_ci:
	cargo fmt -- --check
	ruff format --check poke-engine-py

test_ci:
	cargo test --no-default-features --features "gen9"

ci: install_ci fmt_ci test_ci
