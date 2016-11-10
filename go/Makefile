
export GOPATH=$(shell pwd)

.PHONY: install clean get-deps

install:
	go install github.com/jonfk/utility-belt/basic-auth
	go install github.com/jonfk/utility-belt/day-of-year
	go install github.com/jonfk/utility-belt/github-analytics
	go install github.com/jonfk/utility-belt/inspection-server
	go install github.com/jonfk/utility-belt/pass-gen
	go install github.com/jonfk/utility-belt/prettify-json

clean:
	rm -rf ./bin/
	rm -rf ./pkg/

get-deps:
	cd src/github.com/jonfk/utility-belt && glide install
