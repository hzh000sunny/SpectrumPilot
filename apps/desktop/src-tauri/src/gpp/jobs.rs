use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupCandidate {
    pub tdoc: String,
    pub source_url: String,
    pub work_group: String,
    pub meeting: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupCandidates {
    pub job_id: String,
    pub query: String,
    pub candidates: Vec<GppLookupCandidate>,
}

#[derive(Debug, Clone)]
pub struct LookupJob {
    pub id: String,
    pub token: CancellationToken,
}

#[derive(Default, Clone)]
pub struct JobRegistry {
    jobs: Arc<Mutex<HashMap<String, CancellationToken>>>,
    candidate_senders: Arc<Mutex<HashMap<String, oneshot::Sender<GppLookupCandidate>>>>,
}

impl JobRegistry {
    pub fn create_job(&self) -> LookupJob {
        let id = Uuid::new_v4().to_string();
        let token = CancellationToken::new();
        self.jobs
            .lock()
            .expect("jobs lock")
            .insert(id.clone(), token.clone());

        LookupJob { id, token }
    }

    pub fn cancel_job(&self, id: String) -> bool {
        let Some(token) = self.jobs.lock().expect("jobs lock").remove(&id) else {
            return false;
        };
        self.candidate_senders
            .lock()
            .expect("candidate senders lock")
            .remove(&id);
        token.cancel();
        true
    }

    pub fn finish_job(&self, id: &str) {
        self.jobs.lock().expect("jobs lock").remove(id);
        self.candidate_senders
            .lock()
            .expect("candidate senders lock")
            .remove(id);
    }

    pub fn register_candidate_waiter(&self, id: &str, sender: oneshot::Sender<GppLookupCandidate>) {
        self.candidate_senders
            .lock()
            .expect("candidate senders lock")
            .insert(id.to_string(), sender);
    }

    pub fn choose_candidate(&self, id: &str, candidate: GppLookupCandidate) -> bool {
        let Some(sender) = self
            .candidate_senders
            .lock()
            .expect("candidate senders lock")
            .remove(id)
        else {
            return false;
        };
        sender.send(candidate).is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupProgress {
    pub job_id: String,
    pub stage: String,
    pub message: String,
    pub progress: Option<u8>,
    pub searched_url_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupJobStarted {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupComplete {
    pub job_id: String,
    pub query: String,
    pub source_url: String,
    pub zip_path: String,
    pub extracted_path: String,
    pub opened_path: Option<String>,
    pub cache_status: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::{GppLookupCandidate, GppLookupCandidates, GppLookupComplete, JobRegistry};

    #[test]
    fn cancel_marks_existing_job_token() {
        let registry = JobRegistry::default();
        let job = registry.create_job();
        assert!(!job.token.is_cancelled());

        assert!(registry.cancel_job(job.id));
        assert!(job.token.is_cancelled());
    }

    #[test]
    fn cancel_unknown_job_returns_false() {
        let registry = JobRegistry::default();
        assert!(!registry.cancel_job("missing".to_string()));
    }

    #[test]
    fn lookup_complete_serializes_cache_status_for_the_ui() {
        let complete = GppLookupComplete {
            job_id: "job-1".to_string(),
            query: "R2-2601401".to_string(),
            source_url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
                .to_string(),
            zip_path:
                "C:/SpectrumPilotWorkspace/3gpp/tdocs/RAN2/TSGR2_133bis/R2-2601401/R2-2601401.zip"
                    .to_string(),
            extracted_path: "C:/SpectrumPilotWorkspace/3gpp/tdocs/RAN2/TSGR2_133bis/R2-2601401"
                .to_string(),
            opened_path: Some(
                "C:/SpectrumPilotWorkspace/3gpp/tdocs/RAN2/TSGR2_133bis/R2-2601401/R2-2601401.docx"
                    .to_string(),
            ),
            cache_status: "cached_document".to_string(),
            message: "Opened cached R2-2601401.".to_string(),
        };

        let value = serde_json::to_value(complete).expect("serialize complete");

        assert_eq!(value["cacheStatus"], "cached_document");
    }

    #[test]
    fn lookup_candidates_payload_serializes_for_the_ui() {
        let candidates = GppLookupCandidates {
            job_id: "job-1".to_string(),
            query: "R2-2601401".to_string(),
            candidates: vec![GppLookupCandidate {
                tdoc: "R2-2601401".to_string(),
                source_url:
                    "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
                        .to_string(),
                work_group: "RAN2".to_string(),
                meeting: "TSGR2_133bis".to_string(),
            }],
        };

        let value = serde_json::to_value(candidates).expect("serialize candidates");

        assert_eq!(value["jobId"], "job-1");
        assert_eq!(value["query"], "R2-2601401");
        assert_eq!(value["candidates"][0]["tdoc"], "R2-2601401");
        assert_eq!(
            value["candidates"][0]["sourceUrl"],
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
        );
        assert_eq!(value["candidates"][0]["workGroup"], "RAN2");
        assert_eq!(value["candidates"][0]["meeting"], "TSGR2_133bis");
    }
}
