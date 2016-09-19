package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"net/http/httputil"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/davecgh/go-spew/spew"
	"github.com/urfave/cli"
	git "gopkg.in/src-d/go-git.v3"
)

const (
	GithubGraphqlUrl   = "https://api.github.com/graphql"
	GithubRateLimitUrl = "https://api.github.com/rate_limit"
)

var (
	httpClient *http.Client
)

func main() {
	app := cli.NewApp()
	app.Name = "github-analytics"
	app.Usage = "Analyzes your github repositories"
	app.Before = func(c *cli.Context) error {
		if c.String("token") == "" {
			return fmt.Errorf("No token passed as argument")
		}
		httpClient = &http.Client{}
		return nil
	}
	app.Action = func(c *cli.Context) error {
		repositories := FetchRepositoriesFromNetOrFile(c.String("token"))

		for _, repo := range repositories {
			AnalyzeGithubRepo(c.String("username"), repo)
		}
		fmt.Printf("Total Count : %d\n", len(repositories))
		return nil
	}
	app.Commands = []cli.Command{
		{
			Name:    "ratelimit",
			Aliases: []string{},
			Usage:   "Check the github ratelimit",
			Action: func(c *cli.Context) error {
				fmt.Println(c.GlobalString("token"))
				GithubCheckRateLimit(c.GlobalString("token"))
				return nil
			},
		},
	}
	app.Flags = []cli.Flag{
		cli.StringFlag{
			Name:  "token,t",
			Usage: "Github access token",
			Value: "",
		},
		cli.StringFlag{
			Name:  "username,u",
			Usage: "Github username",
			Value: "",
		},
	}

	app.Run(os.Args)
}

func getAllGithubRepositories(githubAccessToken string) []Repository {
	var repositories []Repository

	query := `
{
  viewer {
    repositories(first: 30%s) {
      pageInfo {
        startCursor
        endCursor
      }
      totalCount
      edges {
        node {
          id
          name
          isFork
          isPrivate
          description
        }
      }
    }
  }
}`
	firstQuery := strings.Replace(fmt.Sprintf(query, ""), "\n", "", -1)
	githubResp := getGithubRepositoriesFromApi(githubAccessToken, firstQuery)

	for len(githubResp.Data.Viewer.Repositories.Edges) > 0 {
		spew.Dump(githubResp)
		for _, edge := range githubResp.Data.Viewer.Repositories.Edges {
			repositories = append(repositories, edge.Node)
		}
		nextQuery := strings.Replace(fmt.Sprintf(query, fmt.Sprintf("after: \"%s\"", githubResp.Data.Viewer.Repositories.PageInfo.EndCursor)), "\n", "", -1)
		time.Sleep(5 * time.Second)
		githubResp = getGithubRepositoriesFromApi(githubAccessToken, nextQuery)

	}

	return repositories
}

func getGithubRepositoriesFromApi(githubAccessToken, query string) GithubQueryResponse {
	queryBody, err := json.Marshal(Query{Query: query})
	if err != nil {
		panic(err)
	}

	req, err := http.NewRequest("POST", GithubGraphqlUrl, bytes.NewReader(queryBody))
	if err != nil {
		panic(err)
	}
	req.Header.Add("Authorization", fmt.Sprintf("bearer %s", githubAccessToken))

	resp, err := httpClient.Do(req)
	if err != nil {
		panic(err)
	}

	if resp.StatusCode != 200 {
		dump, err := httputil.DumpResponse(resp, true)
		if err != nil {
			panic(err)
		}
		panic(string(dump))
	}

	remaining := resp.Header.Get("X-Ratelimit-Remaining")
	if remainingI, _ := strconv.Atoi(remaining); remainingI < 2 {
		reset := resp.Header.Get("X-Ratelimit-Reset")
		resetI, _ := strconv.Atoi(reset)
		resetDate := time.Unix(int64(resetI), 0)
		GithubCheckRateLimit(githubAccessToken)
		panic(fmt.Sprintf("No more github API Calls until %s", resetDate))
	}

	respBody, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}

	githubResp := GithubQueryResponse{}

	err = json.Unmarshal(respBody, &githubResp)
	if err != nil {
		panic(err)
	}
	return githubResp
}

type Query struct {
	Query string `json:"query"`
}

type GithubQueryResponse struct {
	Data struct {
		Viewer struct {
			Repositories struct {
				PageInfo struct {
					StartCursor string `json:"startCursor"`
					EndCursor   string `json:"endCursor"`
				} `json:"pageInfo"`
				TotalCount int `json:"totalCount"`
				Edges      []struct {
					Node Repository `json:"node"`
				} `json:"edges"`
			} `json:"repositories"`
		} `json:"viewer"`
	} `json:"data"`
	Errors []struct {
		Message string `json:"message"`
	} `json:"errors"`
}

type Repository struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	IsFork      bool   `json:"isFork"`
	IsPrivate   bool   `json:"isPrivate"`
	Description string `json:"description"`
}

func AnalyzeGithubRepo(username string, repo Repository) {
	if repo.IsFork {
		return
	}
	repoUrl := ToGithubGitHttpsUrl(username, repo.Name)
	r, err := git.NewRepository(repoUrl, nil)
	if err != nil {
		panic(err)
	}

	if err := r.PullDefault(); err != nil {
		return
		//panic(err)
	}

	iter, err := r.Commits()
	if err != nil {
		panic(err)
	}
	defer iter.Close()

	var commits []git.Commit

	for {
		//the commits are not shorted in any special order
		commit, err := iter.Next()
		if err != nil {
			if err == io.EOF {
				break
			}

			panic(err)
		}

		commits = append(commits, *commit)
	}
	sort.Sort(ByTime(commits))
	// TODO complete analysis print the commit properly and something smarter with frequency and recent commits
	fmt.Printf("* %s\n\t* %s\n\t* %s\n\t* Commits:\n\t\t* First %s\n\t\t* Last %s\n", repo.Name, repoUrl, repo.Description, commits[0].Author.When, commits[len(commits)-1].Author.When.String())
}

func ToGithubGitHttpsUrl(username, repoName string) string {
	return fmt.Sprintf("https://github.com/%s/%s", username, repoName)
}

type ByTime []git.Commit

func (a ByTime) Len() int           { return len(a) }
func (a ByTime) Swap(i, j int)      { a[i], a[j] = a[j], a[i] }
func (a ByTime) Less(i, j int) bool { return a[i].Author.When.Before(a[j].Author.When) }

func SaveRepositoriesToFile(repositories []Repository, filename string) {
	repositoriesByte, err := json.MarshalIndent(repositories, "", "  ")
	if err != nil {
		panic(err)
	}
	err = ioutil.WriteFile(filename, repositoriesByte, 0644)
	if err != nil {
		panic(err)
	}
}

func FetchRepositoriesFromNetOrFile(token string) []Repository {
	filename := "./repositories.json"
	if _, err := os.Stat(filename); os.IsNotExist(err) {
		repositories := getAllGithubRepositories(token)
		SaveRepositoriesToFile(repositories, filename)
		return repositories
	} else {
		repositoriesByte, err := ioutil.ReadFile(filename)
		if err != nil {
			panic(err)
		}

		var repositories []Repository
		err = json.Unmarshal(repositoriesByte, &repositories)
		if err != nil {
			panic(err)
		}
		return repositories

	}
}

func GithubCheckRateLimit(token string) GithubRateLimitModel {
	req, err := http.NewRequest("GET", GithubRateLimitUrl, nil)
	if err != nil {
		panic(err)
	}
	if token != "" {
		req.Header.Add("Authorization", fmt.Sprintf("bearer %s", token))
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		panic(err)
	}

	if resp.StatusCode != 200 {
		dump, err := httputil.DumpResponse(resp, true)
		if err != nil {
			panic(err)
		}
		panic(string(dump))
	}

	rateLimitBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}

	rateLimit := GithubRateLimitModel{}
	err = json.Unmarshal(rateLimitBytes, &rateLimit)
	if err != nil {
		panic(err)
	}
	spew.Dump(rateLimit)
	return rateLimit
}

type GithubRateLimitModel struct {
	Resources struct {
		Core struct {
			Limit     int `json:"limit"`
			Remaining int `json:"remaining"`
			Reset     int `json:"reset"`
		} `json:"core"`
		Search struct {
			Limit     int `json:"limit"`
			Remaining int `json:"remaining"`
			Reset     int `json:"reset"`
		} `json:"search"`
		Graphql struct {
			Limit     int `json:"limit"`
			Remaining int `json:"remaining"`
			Reset     int `json:"reset"`
		} `json:"graphql"`
	} `json:"resources"`
}
