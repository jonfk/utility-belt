package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/urfave/cli"
)

const (
	DateLayout = "2006-01-02"
)

func main() {
	app := cli.NewApp()
	app.Name = "day-of-year"
	app.Usage = "Get the day of the year for journal entries"
	app.Action = func(c *cli.Context) error {
		args := os.Args[1:]
		if len(args) > 0 {
			for _, d := range args {
				day, err := parseDate(d)
				if err != nil {
					fmt.Println(err)
					return err
				}
				fmt.Println(getDateMessage(day))
			}
		} else {
			day := time.Now()
			fmt.Println(getDateMessage(day))
		}
		return nil
	}

	app.Commands = []cli.Command{
		{
			Name:    "commit",
			Aliases: []string{"c"},
			Usage:   "commit file with day and date",
			Action: func(c *cli.Context) error {
				file := c.Args().First()
				day, err := parseDate(file)
				if err != nil {
					fmt.Println(err)
					return err
				}
				err = commitFile(file, getDateMessage(day))
				if err != nil {
					fmt.Println(err)
					return err
				}

				return nil
			},
		},
	}

	app.Run(os.Args)

}

func getDateMessage(date time.Time) string {
	return fmt.Sprintf("Day %d: %s", date.YearDay(), date.Format(DateLayout))
}

func parseDate(dateStr string) (time.Time, error) {
	if withFileExt := strings.Split(dateStr, "."); len(withFileExt) > 1 {
		dateStr = withFileExt[0]
	}
	var (
		date time.Time
		err  error
	)
	date, err = time.Parse(DateLayout, dateStr)
	if err != nil {
		return date, err
	}
	return date, nil
}

func commitFile(file, message string) error {
	out, err := exec.Command("git", "add", file).Output()
	if err != nil {
		return err
	}
	fmt.Println(string(out))

	out, err = exec.Command("git", "commit", "-m", message).Output()
	if err != nil {
		return err
	}
	fmt.Println(string(out))

	return nil
}
