
# prune-openapi

Small CLI utility that lets you pipe an OpenAPI JSON schema through `fzf` and interactively choose operations.

## IDEA

I would like to brainstorm a python cli program that can take a path to an openapi json schema file and optionally an output yaml file. If the output file is not provided it defaults to pruned-openapi.yaml.

It should get a list of all paths and operations in the openapi spec. Each set of (path, method, operationId and summary) would be a selectable operation.
The list of operations would be selectable in fzf with the operationId and summary having priority to be shown/presented to the user.
Since operationId and summary are optional in openapi schemas, we need to be able to handle this.

Finally print the operations selected in fzf. If a selected operation doesn't have an operationId defined, it should return an error and exit.

Then use the following command with the operations selected to produce the output file. `pnpm dlx openapi-extract -o OPERATION -- openapi.json output.json`

fzf would be used by shelling out to it. The script should verify that fzf binary exists in the PATH, if it doesn't exist it would return an error and exit.
It would be implemented in main.py
This is a uv project. Use dependencies judiciously for best practice packages.
This should mainly support openapi 3 versions.


```bash
Usage: openapi-extract [options] {infile} [{outfile}]

Options:
  -h, --help             Show help                                     [boolean]
  --version              Show version number                           [boolean]
  --openai               make the definition OpenAI compliant          [boolean]
  --server               include server information                    [boolean]
  --shard                shard the input to an output directory         [string]
  -p, --path             the path to extract                            [string]
  -o, --operationid      the operationIds to extract                     [array]
  -m, --method           the method to extract for the given path       [string]
  -i, --info             copy full info object, otherwise minimal      [boolean]
  -d, --removeDocs       remove all externalDocs properties            [boolean]
  -r, --removeExamples   remove all example/examples properties        [boolean]
  -x, --removeExtensions remove all x- extension properties            [boolean]
  -s, --security         include security information                  [boolean]
  -v, --verbose          increase verbosity                            [boolean]
```

Redocly cli to output to yaml otherwise keep the json outputted by openapi-extract
```
pnpm --package=@redocly/cli dlx redocly bundle tmp.json \
  --remove-unused-components \
  --ext yaml \
  -o slim.yaml
```

Allow non-interactive selection by taking in a list of operationIds. If it is provided on the cli, we skip the collection of operations and fzf and go straight to post processing. 

Workflow

- Validate inputs: check file exists/readable, confirm JSON/YAML extension, default pruned-openapi.yaml when --output absent, verify fzf in PATH.
- Load spec: parse JSON (or detect YAML and convert first), confirm OpenAPI version 3.x, normalize keys, capture paths entries.
- Build operation catalogue: iterate each path + HTTP method, extract operationId, summary, description fallback, note missing IDs.
- Format for fzf: compose display string (operationId – summary [METHOD PATH]), pass via stdin, allow multi-select, guard against nothing selected.

- Post-process selections: ensure each chosen entry has operationId, filter original spec to only those operations, prune empty containers (methods, paths, tags).
- Output/save: use redocly cli for output
- Error handling: actionable messages for missing fzf, invalid spec, no selections, missing IDs, write failures.


## TODO

- replace openapi-extract with own implementation

