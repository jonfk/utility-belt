package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"io/ioutil"
	"log"
	"os"
)

var write bool
var lerr *log.Logger

func init() {
	const usage = "overwrite to file"
	flag.BoolVar(&write, "write", false, usage)
	flag.BoolVar(&write, "w", false, usage+" (shorthand)")

	lerr = log.New(os.Stderr, "", 0)

	flag.Parse()
}

func main() {
	args := flag.Args()
	if len(args) < 1 {
		lerr.Fatal("Prettifies json")
	}
	filename := args[0]

	unformattedJson, err := ioutil.ReadFile(filename)
	if err != nil {
		lerr.Fatal(err)
	}

	var out bytes.Buffer
	err = json.Indent(&out, unformattedJson, "", "  ")
	if err != nil {
		lerr.Fatal(err)
	}

	if write {
		ioutil.WriteFile(filename, out.Bytes(), 0777)
	} else {
		out.WriteTo(os.Stdout)
	}

}
