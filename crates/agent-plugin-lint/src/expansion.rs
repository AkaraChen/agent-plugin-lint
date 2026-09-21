//! The specification deliberately defines only these two, one-pass substitutions.
pub(crate) fn once(source: &str, root: &str, data: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(index) = rest.find("${PLUGIN_") {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some(tail) = rest.strip_prefix("${PLUGIN_ROOT}") {
            out.push_str(root);
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("${PLUGIN_DATA}") {
            out.push_str(data);
            rest = tail;
        } else {
            // Copy one byte so unrecognized placeholder-like text remains literal.
            out.push_str(&rest[..1]);
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::once;
    #[test]
    fn replacement_is_not_scanned_again() {
        assert_eq!(
            once("${PLUGIN_ROOT}", "${PLUGIN_DATA}", "D"),
            "${PLUGIN_DATA}"
        );
        assert_eq!(once("x${OTHER}", "R", "D"), "x${OTHER}");
    }

    #[test]
    fn all_replacements_are_single_pass() {
        assert_eq!(
            once(
                "${PLUGIN_ROOT}${PLUGIN_ROOT}${UNKNOWN}",
                "${PLUGIN_DATA}",
                "D"
            ),
            "${PLUGIN_DATA}${PLUGIN_DATA}${UNKNOWN}"
        );
    }
}
