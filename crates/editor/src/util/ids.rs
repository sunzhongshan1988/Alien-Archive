pub(crate) fn next_editor_object_id(prefix: &str, instances: &[content::ObjectInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

pub(crate) fn next_editor_entity_id(prefix: &str, instances: &[content::EntityInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

pub(crate) fn next_editor_zone_id(prefix: &str, instances: &[content::ZoneInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

pub(crate) fn ground_selection_id(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

pub(crate) fn parse_ground_selection_id(id: &str) -> Option<[i32; 2]> {
    let (x, y) = id.split_once(',')?;
    Some([x.parse().ok()?, y.parse().ok()?])
}
