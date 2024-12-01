pub(crate) fn mbtile_project_y(x: i64, y: i64, z: i64) -> i64 {
    (1 << z) - 1 - y
}