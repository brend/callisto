CALLISTO ?= ../target/debug/callisto
PDC ?= pdc
PDX ?= BouncingBall.pdx

build:
	$(CALLISTO) build src/game.cal --config callisto.toml -o Source/
	$(PDC) Source/ $(PDX)

run: build
	open $(PDX)

build-lua:
	$(CALLISTO) build src/game.cal --config callisto.toml -o Source/
