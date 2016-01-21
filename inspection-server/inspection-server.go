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

	// http.HandleFunc("/bar", func(w http.ResponseWriter, r *http.Request) {
	// 	fmt.Fprintf(w, "Hello, %q", html.EscapeString(r.URL.Path))
	// })

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

	// newURL, err := url.Parse("https://www.strava.com/api/v3/uploads")
	// if err != nil {
	// 	log.Println(err)
	// }

	// newReq, err := http.ReadRequest(bufio.NewReader(strings.NewReader(buf.String())))
	// newReq.RequestURI = ""
	// newReq.URL = newURL

	// client := &http.Client{}
	// resp, err := client.Do(newReq)
	// if err != nil {
	// 	log.Println(err)
	// }
	// log.Println("response: ")
	// spew.Dump(resp)

	// if resp != nil {
	// 	respBuf := new(bytes.Buffer)
	// 	respBuf.ReadFrom(resp.Body)
	// 	respBody := respBuf.String()
	// 	fmt.Println(respBody)
	// }

}
