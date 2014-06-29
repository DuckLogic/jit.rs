RUSTC ?= rustc
CARGO ?= cargo
RUSTDOC ?= rustdoc
.PHONY: all build doc update-doc clean
all: build doc
build:
	$(CARGO) build
doc: build
	rm -rf doc
	$(RUSTDOC) src/libjit/jit.rs -o doc -L target/deps
update-doc: doc
	rm -rf /tmp/doc
	mv doc /tmp/doc
	git checkout gh-pages
	rm -rf ./*
	mv /tmp/doc/* .
	-git add -A .
	-git commit -a -m "Auto-update docs"
	-git push origin gh-pages
	git checkout master
clean:
	rm -rf target/*jit*
