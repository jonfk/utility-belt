package main

import (
	"fmt"
	"os"
	"time"
)

const DateLayout = "2006-01-02"

func main() {
	args := os.Args[1:]
	if len(args) > 0 {
		for _, d := range args {
			day, err := time.Parse(DateLayout, d)
			if err != nil {
				fmt.Println(err)
			}
			fmt.Println(day.YearDay())
		}
	} else {
		day := time.Now().YearDay()
		fmt.Println(day)
	}

}
