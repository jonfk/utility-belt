package main

import (
	// "bufio"
	// "github.com/davecgh/go-spew/spew"
	// "net/url"
	// "strings"
	"bytes"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
)

func main() {
	http.HandleFunc("/", handler)

	fmt.Println("serving on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

func handler(w http.ResponseWriter, r *http.Request) {
	//spew.Dump(r)

	fmt.Println("Body:")

	buf := new(bytes.Buffer)

	r.Write(buf)

	//buf.ReadFrom(r.Body)
	reqStr := buf.String()
	fmt.Println(reqStr)

	ioutil.WriteFile("temp.txt", buf.Bytes(), 0777)
	fmt.Fprintf(w, "ok printed")

}
