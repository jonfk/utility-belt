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
	"strings"

	"github.com/urfave/cli"
	git "gopkg.in/src-d/go-git.v3"
)

const (
	GithubGraphqlUrl = "https://api.github.com/graphql"
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

	for githubResp.Data.Viewer.Repositories.TotalCount > 0 {
		for _, edge := range githubResp.Data.Viewer.Repositories.Edges {
			repositories = append(repositories, edge.Node)
		}
		nextQuery := strings.Replace(fmt.Sprintf(query, fmt.Sprintf("after: \"%s\"", githubResp.Data.Viewer.Repositories.PageInfo.EndCursor)), "\n", "", -1)
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
	repoUrl := ToGithubGitHttpsUrl(username, repo.Name)
	r, err := git.NewRepository(repoUrl, nil)
	if err != nil {
		panic(err)
	}

	if err := r.PullDefault(); err != nil {
		panic(err)
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
	fmt.Printf("%s %s %t First Commit : %s Last: %s \n", repo.Name, repo.Description, repo.IsFork, commits[0].String(), commits[len(commits)-1].String())
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
