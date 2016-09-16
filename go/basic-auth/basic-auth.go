package main

import (
	"encoding/base64"
	"fmt"
	"os"
)

func main() {
	args := os.Args[1:]
	if len(args) < 2 {
		fmt.Println("basic-auth takes a username and password and returns a basic auth header")
	}

	username, password := args[0], args[1]
	fmt.Printf("Authorization: Basic %s\n", basicAuth(username, password))
}

func basicAuth(username, password string) string {
	auth := username + ":" + password
	return base64.StdEncoding.EncodeToString([]byte(auth))
}
