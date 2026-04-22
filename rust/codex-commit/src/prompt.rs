const BASE_PROMPT_PREFIX: &str =
    "Follow these instructions exactly and return only a schema-compliant JSON response.\n\n";

pub fn build_prompt(skill_text: &str, extra_context: &str) -> String {
    let mut prompt = String::from(BASE_PROMPT_PREFIX);
    prompt.push_str(skill_text);

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
