const BASE_PROMPT_PREFIX: &str =
    "Follow these instructions exactly and return only a schema-compliant JSON response.\n\n";
const COMMIT_PROPOSAL_PROMPT: &str = include_str!("../assets/commit-proposal-prompt.md");
const USER_CONTEXT_PLACEHOLDER: &str = "{{USER_CONTEXT_BLOCK}}";

pub fn build_prompt(extra_context: &str) -> String {
    let trimmed_context = extra_context.trim();
    let rendered_prompt = COMMIT_PROPOSAL_PROMPT.trim_end().replace(
        USER_CONTEXT_PLACEHOLDER,
        &render_user_context_block(trimmed_context),
    );

    let mut prompt = String::from(BASE_PROMPT_PREFIX);
    prompt.push_str(rendered_prompt.trim_end());
    prompt
}

fn render_user_context_block(extra_context: &str) -> String {
    if extra_context.is_empty() {
        return String::new();
    }

    format!(
        "## User-Provided Commit Context\n<<<USER_CONTEXT>>>\n{extra_context}\n<<<END_USER_CONTEXT>>>"
    )
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
