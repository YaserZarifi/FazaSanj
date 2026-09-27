//! Texts for tray notifications (low space and the weekly check), in both languages.

use fazasanj_model::Language;

const GB: f64 = 1024.0 * 1024.0 * 1024.0;

fn persian_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32(0x06F0 + (c as u32 - '0' as u32)).unwrap_or(c),
            '.' => '٫',
            _ => c,
        })
        .collect()
}

/// "23.4 GB" or "۲۳٫۴ گیگابایت".
pub fn size_text(bytes: u64, lang: Language) -> String {
    let gb = bytes as f64 / GB;
    let num = if gb >= 100.0 { format!("{gb:.0}") } else { format!("{gb:.1}") };
    match lang {
        Language::En => format!("{num} GB"),
        Language::Fa => format!("{} گیگابایت", persian_digits(&num)),
    }
}

/// Title and body for "drive X is almost full".
pub fn low_space(letter: &str, free: u64, lang: Language) -> (String, String) {
    let size = size_text(free, lang);
    match lang {
        Language::En => (
            format!("{letter} is almost full"),
            format!("Only {size} left. Open Fazasanj to see what is taking the space."),
        ),
        Language::Fa => (
            format!("درایو {letter} تقریباً پر است"),
            format!("فقط {size} جا مانده. فضاسنج را باز کنید تا ببینید چه چیزی جا گرفته."),
        ),
    }
}

/// Title and body for the weekly check. `change` is free space now minus a week ago.
pub fn weekly(letter: &str, free: u64, change: i64, lang: Language) -> (String, String) {
    let free_text = size_text(free, lang);
    let delta = size_text(change.unsigned_abs(), lang);
    match lang {
        Language::En => {
            let title = format!("Weekly check: {letter}");
            let body = if change < 0 {
                format!("{letter} lost {delta} of free space this week. {free_text} is free now.")
            } else {
                format!("{letter} has {free_text} free, {delta} more than last week.")
            };
            (title, body)
        }
        Language::Fa => {
            let title = format!("بررسی هفتگی: {letter}");
            let body = if change < 0 {
                format!("این هفته {delta} از فضای خالی {letter} کم شد. الان {free_text} خالی است.")
            } else {
                format!("{letter} الان {free_text} جای خالی دارد، {delta} بیشتر از هفته قبل.")
            };
            (title, body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_in_both_languages() {
        let b = (23.4 * GB) as u64 + 1;
        assert_eq!(size_text(b, Language::En), "23.4 GB");
        assert_eq!(size_text(b, Language::Fa), "۲۳٫۴ گیگابایت");
        assert_eq!(size_text(250 * GB as u64, Language::En), "250 GB");
    }

    #[test]
    fn weekly_says_lost_or_gained() {
        let (_, body) = weekly("C:", 10 * GB as u64, -(2 * GB as i64), Language::En);
        assert!(body.contains("lost 2.0 GB"));
        let (_, body) = weekly("C:", 10 * GB as u64, 2 * GB as i64, Language::En);
        assert!(body.contains("more than last week"));
    }
}
