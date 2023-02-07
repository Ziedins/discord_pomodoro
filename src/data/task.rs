use chrono::{Duration, Local, NaiveDateTime};

pub struct Task {
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,

    pub description: String,
}
