CALLISTO ?= ../target/debug/callisto
PDC ?= pdc
PDX ?= AutoBootstrap.pdx

build-lua:
	$(CALLISTO) build src/game.cal --config callisto.toml -o Source/ --playdate-bootstrap

build: build-lua
	$(PDC) Source/ $(PDX)

run: build
	open $(PDX)
