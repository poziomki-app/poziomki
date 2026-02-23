use uuid::Uuid;

pub(in crate::api) fn parse_tag_uuids(raw: Option<Vec<String>>) -> Vec<Uuid> {
    raw.unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect()
}
