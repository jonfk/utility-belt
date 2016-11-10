
export GOPATH=$(shell pwd)

.PHONY: install build clean get-deps

build:
	go install github.com/jonfk/utility-belt/basic-auth
	go install github.com/jonfk/utility-belt/day-of-year
	go install github.com/jonfk/utility-belt/github-analytics
	go install github.com/jonfk/utility-belt/inspection-server
	go install github.com/jonfk/utility-belt/pass-gen
	go install github.com/jonfk/utility-belt/prettify-json

install: build
	mkdir -p ~/bin
	mv ./bin/basic-auth ~/bin
	mv ./bin/day-of-year ~/bin
	mv ./bin/github-analytics ~/bin
	mv ./bin/inspection-server ~/bin
	mv ./bin/pass-gen ~/bin
	mv ./bin/prettify-json ~/bin

clean:
	rm -rf ./bin/
	rm -rf ./pkg/

clean-path:
	rm ~/bin/basic-auth
	rm ~/bin/day-of-year
	rm ~/bin/github-analytics
	rm ~/bin/inspection-server
	rm ~/bin/pass-gen
	rm ~/bin/prettify-json

get-deps:
	cd src/github.com/jonfk/utility-belt && glide install
