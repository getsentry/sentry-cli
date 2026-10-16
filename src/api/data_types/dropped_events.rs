use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DroppedEventsBucket {
    pub outcome: String,
    pub reason: String,
    pub category: String,
    pub count: f64,
}

#[derive(Debug, Deserialize)]
pub struct DroppedEventsResponse {
    #[serde(rename = "droppedEvents")]
    pub dropped_events: Vec<DroppedEventsBucket>,
    #[serde(rename = "acceptedEvents", default)]
    pub accepted_events: Vec<DroppedEventsBucket>,
}

pub fn outcome_label(outcome: &str) -> &str {
    match outcome {
        "client_discard" => "Client Discard",
        "filtered" => "Inbound Filter",
        "invalid" => "Invalid or Malformed",
        "rate_limited" => "Rate Limited",
        "abuse" => "Abuse Limit",
        "cardinality_limited" => "Cardinality Limit",
        other => other,
    }
}
