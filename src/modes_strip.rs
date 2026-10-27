// The registry strips, shared by `build.rs` (via `include!`) and the lib's
// tests (via `mod modes_strip`), so the rows a build deletes are unit-tested
// rather than trusted. Build scripts avoid a serde dependency, so these work on
// the file's text and keep its own formatting: a diff of a staged registry
// against the source shows only removals.

/// Keep the rows `keep` accepts, by text, from the registry's `"modes"` array.
///
/// Rebuilds the `modes` array from the entries that survive rather than
/// splicing text around the ones that go. The first attempt did comma
/// surgery -- delete the entry, drop the comma before it AND the one after
/// it -- which removed two separators for one entry and emitted JSON that
/// would not parse. The registry tests caught it immediately, which is the
/// argument for keeping data this load-bearing under test.
#[allow(dead_code)]
pub fn strip_modes(src: &str, keep: impl Fn(&str) -> bool) -> String {
    let Some(arr) = src.find("\"modes\"").and_then(|i| src[i..].find('[').map(|j| i + j)) else {
        return src.to_string();
    };
    let bytes = src.as_bytes();
    let mut entries: Vec<&str> = Vec::new();
    let (mut i, mut depth, mut start) = (arr + 1, 0i32, 0usize);
    let mut arr_end = src.len();
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    entries.push(&src[start..=i]);
                }
            }
            b']' if depth == 0 => {
                arr_end = i;
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let kept: Vec<&str> = entries.into_iter().filter(|e| keep(e)).collect();
    format!("{}\n    {}\n  {}", &src[..=arr], kept.join(",\n    "), &src[arr_end..])
}

/// The text of a row's top-level array field, `"field": [ ... ]`, if present.
fn array_field<'a>(entry: &'a str, field: &str) -> Option<&'a str> {
    let at = entry.find(&format!("\"{field}\""))?;
    let rest = &entry[at..];
    let open = rest.find('[')?;
    // A string array cannot hold a ']' in this registry; the field ends at the first.
    rest[open..].find(']').map(|close| &rest[..open + close])
}

/// The site build (D1): a row that does not list "web" is deleted.
#[allow(dead_code)]
pub fn keeps_on_web(entry: &str) -> bool {
    array_field(entry, "platforms").map(|b| b.contains("\"web\"")).unwrap_or(true)
}

/// CC-HUB-NAV AE-1 F2: the Education build. A row without `editions` ships in
/// both; a row with it ships here only if it names "education".
#[allow(dead_code)]
pub fn keeps_in_education(entry: &str) -> bool {
    array_field(entry, "editions").map(|b| b.contains("\"education\"")).unwrap_or(true)
}
