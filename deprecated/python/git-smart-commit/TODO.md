- [ ] Add README.md and AGENTS.md
- [ ] Add verbose mode
- [x] extended context mode
- [ ] UI improvements:
    - a spinning wheel or some other indicator to show that the llm is working
    - A nicer prompt to the user when choosing y/n to edit. Maybe a chooser that can be selected up and down, etc
    - A nicer way to show what model is being used than
```
ℹ Using model: gemini-3-flash-preview
ℹ Generating commit message for unstaged changes...
```

- [ ] Add tools support to the app. This would allow multi turn flows and allow the llm to gather additional information using the tools
    - Examples of tools to support:
        - Read
        - Glob
        - Search
        - Ls
        - etc
    - A few ways to implement this:
        - Use the actual tool support from the APIs. This may require using litellm or another backend to support this.
        - Use the structured output and extend it with tools.
- [ ] Add an interactive mode. Instead of just running the output from the command, we would put the user in a prompt style of UI and ask for feedback from the user
