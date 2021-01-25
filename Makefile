SHELL := /bin/bash
SN_NODE_VERSION := $(shell grep "^version" < Cargo.toml | head -n 1 | awk '{ print $$3 }' | sed 's/\"//g')
UNAME_S := $(shell uname -s)
DEPLOY_PATH := deploy
DEPLOY_PROD_PATH := ${DEPLOY_PATH}/prod
DEPLOY_CHAOS_PATH := ${DEPLOY_PATH}/chaos

build:
ifeq ($(UNAME_S),Linux)
	@echo "This target should not be used for Linux - please use the `musl` target."
	@exit 1
endif
	rm -rf artifacts
	mkdir artifacts
	cargo build --release
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

musl:
ifneq ($(UNAME_S),Linux)
	@echo "This target only applies to Linux - please use the `build` target."
	@exit 1
endif
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	sudo apt update -y && sudo apt install -y musl-tools
	rustup target add x86_64-unknown-linux-musl
	cargo build --release --target x86_64-unknown-linux-musl --verbose
	find target/x86_64-unknown-linux-musl/release \
		-maxdepth 1 -type f -exec cp '{}' artifacts \;

build-chaos:
ifeq ($(UNAME_S),Linux)
	@echo "This target should not be used for Linux - please use the `musl-chaos` target."
	@exit 1
endif
	rm -rf artifacts
	mkdir artifacts
	cargo build --release --features=chaos,simulated-payouts
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

musl-chaos:
ifneq ($(UNAME_S),Linux)
	@echo "This target only applies to Linux - please use the `build-chaos` target."
	@exit 1
endif
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	sudo apt update -y && sudo apt install -y musl-tools
	rustup target add x86_64-unknown-linux-musl
	cargo build --release --features=chaos,simulated-payouts --target x86_64-unknown-linux-musl --verbose
	find target/x86_64-unknown-linux-musl/release \
		-maxdepth 1 -type f -exec cp '{}' artifacts \;

.ONESHELL:
package-prod-version-artifacts-for-deploy:
	rm -f *.zip *.tar.gz
	rm -rf ${DEPLOY_PATH}
	mkdir -p ${DEPLOY_PROD_PATH}

	zip -j sn_node-${SN_NODE_VERSION}-x86_64-unknown-linux-musl.zip \
		artifacts/prod/x86_64-unknown-linux-musl/release/sn_node
	zip -j sn_node-latest-x86_64-unknown-linux-musl.zip \
		artifacts/prod/x86_64-unknown-linux-musl/release/sn_node
	zip -j sn_node-${SN_NODE_VERSION}-x86_64-pc-windows-msvc.zip \
		artifacts/prod/x86_64-pc-windows-msvc/release/sn_node.exe
	zip -j sn_node-latest-x86_64-pc-windows-msvc.zip \
		artifacts/prod/x86_64-pc-windows-msvc/release/sn_node.exe
	zip -j sn_node-${SN_NODE_VERSION}-x86_64-apple-darwin.zip \
		artifacts/prod/x86_64-apple-darwin/release/sn_node
	zip -j sn_node-latest-x86_64-apple-darwin.zip \
		artifacts/prod/x86_64-apple-darwin/release/sn_node

	tar -C artifacts/prod/x86_64-unknown-linux-musl/release \
		-zcvf sn_node-${SN_NODE_VERSION}-x86_64-unknown-linux-musl.tar.gz sn_node
	tar -C artifacts/prod/x86_64-unknown-linux-musl/release \
		-zcvf sn_node-latest-x86_64-unknown-linux-musl.tar.gz sn_node
	tar -C artifacts/prod/x86_64-pc-windows-msvc/release \
		-zcvf sn_node-${SN_NODE_VERSION}-x86_64-pc-windows-msvc.tar.gz sn_node.exe
	tar -C artifacts/prod/x86_64-pc-windows-msvc/release \
		-zcvf sn_node-latest-x86_64-pc-windows-msvc.tar.gz sn_node.exe
	tar -C artifacts/prod/x86_64-apple-darwin/release \
		-zcvf sn_node-${SN_NODE_VERSION}-x86_64-apple-darwin.tar.gz sn_node
	tar -C artifacts/prod/x86_64-apple-darwin/release \
		-zcvf sn_node-latest-x86_64-apple-darwin.tar.gz sn_node

	mv *.zip ${DEPLOY_PROD_PATH}
	mv sn_node-${SN_NODE_VERSION}-*.tar.gz ${DEPLOY_PROD_PATH}
	mv sn_node-latest-*.tar.gz ${DEPLOY_PROD_PATH}

.ONESHELL:
package-chaos-version-artifacts-for-deploy:
	rm -rf ${DEPLOY_CHAOS_PATH}
	mkdir -p ${DEPLOY_CHAOS_PATH}

	tar -C artifacts/chaos/x86_64-unknown-linux-musl/release \
		-zcvf sn_node-CHAOS-${SN_NODE_VERSION}-x86_64-unknown-linux-musl.tar.gz sn_node
	tar -C artifacts/chaos/x86_64-unknown-linux-musl/release \
		-zcvf sn_node-CHAOS-latest-x86_64-unknown-linux-musl.tar.gz sn_node
	tar -C artifacts/chaos/x86_64-pc-windows-msvc/release \
		-zcvf sn_node-CHAOS-${SN_NODE_VERSION}-x86_64-pc-windows-msvc.tar.gz sn_node.exe
	tar -C artifacts/chaos/x86_64-pc-windows-msvc/release \
		-zcvf sn_node-CHAOS-latest-x86_64-pc-windows-msvc.tar.gz sn_node.exe
	tar -C artifacts/chaos/x86_64-apple-darwin/release \
		-zcvf sn_node-CHAOS-${SN_NODE_VERSION}-x86_64-apple-darwin.tar.gz sn_node
	tar -C artifacts/chaos/x86_64-apple-darwin/release \
		-zcvf sn_node-CHAOS-latest-x86_64-apple-darwin.tar.gz sn_node

	mv sn_node-CHAOS-*.tar.gz ${DEPLOY_CHAOS_PATH}
