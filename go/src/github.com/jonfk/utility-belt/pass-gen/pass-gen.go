package main

import (
	"bytes"
	"crypto/rand"
	"fmt"
	"log"
	"math/big"
	"os"

	"github.com/urfave/cli"
)

const DefaultLength = 16

func init() {
	log.SetPrefix("")
	cli.VersionFlag = cli.BoolFlag{Name: "version, V"}
}

func main() {
	app := cli.NewApp()
	app.Name = "pass-gen"
	app.Usage = "Generates a random password"
	app.Before = func(c *cli.Context) error {
		return nil
	}
	app.Action = func(c *cli.Context) error {

		length := c.Int("length")
		excludedTypes := []CharType{}
		excludedChars := []int32{}

		if c.Bool("special") {
			excludedTypes = append(excludedTypes, SpecialCharType)
		}
		if c.Bool("number") {
			excludedTypes = append(excludedTypes, NumberCharType)
		}
		if c.Bool("upper") {
			excludedTypes = append(excludedTypes, UpperCharType)
		}
		if c.Bool("lower") {
			excludedTypes = append(excludedTypes, LowerCharType)
		}
		for _, ch := range c.String("exclude") {
			excludedChars = append(excludedChars, int32(ch))
		}

		if c.Bool("verbose") {
			fmt.Printf("Characters to be excluded:")
			for _, ch := range excludedChars {
				fmt.Printf(" %c", rune(ch))
			}
			fmt.Println()
		}

		randInts, err := GenerateRandomInts(length, excludedChars, excludedTypes)
		if err != nil {
			log.Fatal(err)
		}

		if c.Bool("verbose") {
			fmt.Printf("Random Ints generated: %v\n", randInts)
		}
		fmt.Printf("%v\n", IntsToString(randInts))
		return nil
	}

	app.Flags = []cli.Flag{
		cli.IntFlag{
			Name:  "length,l",
			Usage: "Password Length",
			Value: DefaultLength,
		},
		cli.BoolFlag{
			Name:  "special,s",
			Usage: "Exclude special characters: !\"#$%&()*+,-./:;<=>?@[\\]^_`{|}~",
		},
		cli.BoolFlag{
			Name:  "number,n",
			Usage: "Exclude numbers",
		},
		cli.BoolFlag{
			Name:  "upper,u",
			Usage: "Exclude uppercase characters",
		},
		cli.BoolFlag{
			Name:  "lower",
			Usage: "Exclude lowercase characters",
		},
		cli.BoolFlag{
			Name:  "verbose, v",
			Usage: "verbose output",
		},
		cli.StringFlag{
			Name:  "exclude, e",
			Usage: "Characters to be excluded",
			Value: "",
		},
	}

	app.Run(os.Args)
}

func IntsToString(nums []int32) string {
	buf := bytes.Buffer{}

	for _, x := range nums {
		buf.WriteRune(rune(x))
	}
	return buf.String()
}

func GenerateRandomInts(length int, excluded []int32, excludedTypes []CharType) ([]int32, error) {
	// Filter characters outside of valid ascii range (no unicode or nonvisible chars)
	toExclude := []int32{}
	for _, x := range excluded {
		if x >= 32 && x < 127 {
			toExclude = append(toExclude, x)
		}
	}

	randInts := []int32{}

	for i := 0; i < length; i++ {

		bigRandNum, err := rand.Int(rand.Reader, big.NewInt(95))
		if err != nil {
			return randInts, fmt.Errorf("Error generating random number: %v", err)
		}
		randNum := int32(bigRandNum.Int64())
		randNum += 32
		if !containsInt32(randNum, toExclude) && !containsCharType(GetCharType(randNum), excludedTypes) {
			randInts = append(randInts, randNum)
		} else {
			i -= 1
		}
	}
	return randInts, nil
}

func containsInt32(a int32, ints []int32) bool {
	for _, x := range ints {
		if x == a {
			return true
		}
	}
	return false
}
