#!/usr/bin/env python3

import requests

token = "652d3644123edbef0fe558f17572b0c0c697b426"
github_graphql_url = "https://api.github.com/graphql"

def main():
    getAllRepositories()

def getAllRepositories():
    headers = {"Authorization": "bearer {}".format(token)}
    query = """
{{
  viewer {{
    repositories(first: 30{}) {{
      pageInfo {{
        startCursor
        endCursor
      }}
      totalCount
      edges {{
        node {{
          id
          name
          isFork
          isPrivate
          description
        }}
      }}
    }}
  }}
}}
"""
    firstQuery = query.format("").replace('\n', ' ')
    # print(firstQuery)
    r = requests.post(github_graphql_url, headers=headers, json={'query': firstQuery})
    print(r.json())

if __name__ == "__main__":
    # execute only if run as a script
    main()
