//! Command line: `--volume C: --pipe <name> --token <hex>`. Strict, because this runs as admin.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub letter: char,
    pub pipe: String,
    pub token: String,
}

fn valid_pipe(name: &str) -> bool {
    let Some(hex) = name.strip_prefix("fazasanj-") else { return false };
    (16..=64).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_token(t: &str) -> bool {
    (16..=128).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut volume = None;
    let mut pipe = None;
    let mut token = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--volume" => &mut volume,
            "--pipe" => &mut pipe,
            "--token" => &mut token,
            other => return Err(format!("unknown argument {other}")),
        };
        if slot.is_some() {
            return Err(format!("{a} given twice"));
        }
        *slot = Some(it.next().ok_or_else(|| format!("{a} needs a value"))?);
    }
    let volume = volume.ok_or("missing --volume")?;
    let b = volume.as_bytes();
    if b.len() != 2 || !b[0].is_ascii_alphabetic() || b[1] != b':' {
        return Err("volume must look like C:".into());
    }
    let pipe = pipe.ok_or("missing --pipe")?;
    if !valid_pipe(&pipe) {
        return Err("bad pipe name".into());
    }
    let token = token.ok_or("missing --token")?;
    if !valid_token(&token) {
        return Err("bad token".into());
    }
    Ok(Args { letter: (b[0] as char).to_ascii_uppercase(), pipe, token })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Args, String> {
        parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn good_args() {
        let a = p("--volume c: --pipe fazasanj-0123456789abcdef --token 00112233445566778899aabbccddeeff")
            .expect("args");
        assert_eq!(a.letter, 'C');
        assert_eq!(a.pipe, "fazasanj-0123456789abcdef");
    }

    #[test]
    fn rejects_anything_odd() {
        let tok = "--token 00112233445566778899aabbccddeeff";
        let pipe = "--pipe fazasanj-0123456789abcdef";
        assert!(p(&format!("--volume C:\\ {pipe} {tok}")).is_err());
        assert!(p(&format!("--volume \\\\.\\C: {pipe} {tok}")).is_err());
        assert!(p(&format!("--volume 1: {pipe} {tok}")).is_err());
        assert!(p(&format!("--volume C: --pipe ..\\..\\evil {tok}")).is_err());
        assert!(p(&format!("--volume C: --pipe C:\\Windows\\x {tok}")).is_err());
        assert!(p(&format!("--volume C: {pipe} --token short")).is_err());
        assert!(p(&format!("--volume C: {pipe} {tok} --extra 1")).is_err());
        assert!(p(&format!("--volume C: --volume D: {pipe} {tok}")).is_err());
        assert!(p("--volume C:").is_err());
    }
}
