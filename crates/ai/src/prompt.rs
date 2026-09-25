//! The instructions sent with every request.

use fazasanj_model::Language;
use serde_json::Value;

const BASE: &str = r#"You help a normal Windows user understand why a folder on their disk is big.
You only get metadata about the folder (path, sizes, file counts, dates, the most common
file extensions and the names of its biggest children). You never see file contents.
Some names may be replaced with placeholders like folder_1 or <user>; do not guess what they hide.

Reply with ONE JSON object and nothing else. No markdown, no code fences, no extra keys:
{"what": string, "why_big": string, "safety": "probably_safe" | "careful" | "do_not_touch", "consequence": string}

- what: what this folder most likely is and which program or feature uses it. One short sentence.
- why_big: why it takes this much space. One or two short sentences.
- safety: how risky it is to delete it.
    probably_safe = caches, temp files or leftovers that are recreated or not needed.
    careful = user data, settings, game or app data, or anything you are not sure about.
    do_not_touch = Windows or program files needed to run, or anything that could break the system.
  Never use "safe". If you are unsure, use "careful".
- consequence: what happens if the user deletes it. One or two short sentences.

Keep every value short, plain and friendly. No technical jargon unless it is needed.
If the metadata is not enough to tell, say so honestly in "what" and use "careful"."#;

const LANG_EN: &str = "Write all values in simple English.";

const LANG_FA: &str = "Write all values in natural, simple Persian that both Iranian and Afghan \
readers understand easily. Use common everyday words (for example فضا، پوشه، فایل، برنامه، حذف) \
and correct Persian punctuation and half-spaces. Keep program and file names in their original \
form. The JSON keys and the safety value stay in English.";

pub fn system_prompt(language: Language) -> String {
    let lang = match language {
        Language::Fa => LANG_FA,
        Language::En => LANG_EN,
    };
    format!("{BASE}\n\n{lang}")
}

pub fn user_message(payload: &Value) -> String {
    let pretty = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    format!("Folder metadata (JSON):\n{pretty}\n\nAnswer with the JSON object only.")
}

pub fn language_code(language: Language) -> &'static str {
    match language {
        Language::Fa => "fa",
        Language::En => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_forbids_safe_and_names_language() {
        let fa = system_prompt(Language::Fa);
        assert!(fa.contains("Never use \"safe\""));
        assert!(fa.contains("Persian"));
        assert!(system_prompt(Language::En).contains("English"));
        let msg = user_message(&serde_json::json!({"path": "C:\\x"}));
        assert!(msg.contains("C:\\\\x"));
    }
}
