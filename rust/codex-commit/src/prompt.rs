const BASE_PROMPT_PREFIX: &str =
    "Follow these instructions exactly and return only a schema-compliant JSON response.\n\n";
const COMMIT_PROPOSAL_PROMPT: &str = include_str!("../assets/commit-proposal-prompt.md");

pub fn build_prompt(extra_context: &str) -> String {
    let mut prompt = String::from(BASE_PROMPT_PREFIX);
    prompt.push_str(COMMIT_PROPOSAL_PROMPT.trim_end());

    let trimmed_context = extra_context.trim();
    if !trimmed_context.is_empty() {
        prompt.push_str("\n\nAdditional user context:\n");
        prompt.push_str(trimmed_context);
    }

    prompt
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
