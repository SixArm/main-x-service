//! Background worker scaffold (loco `BackgroundWorker`).
//!
//! A placeholder async job carried over from the loco template; not yet
//! wired to real work. Kept so the queue registration in
//! [`crate::app`] has a concrete worker to register.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Background worker that performs asynchronous download-style jobs.
pub struct DownloadWorker {
    /// The shared loco application context (database, queue, config).
    pub ctx: AppContext,
}

/// Arguments for a [`DownloadWorker`] job.
#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    /// Identifier of the subject the job operates on.
    pub user_guid: String,
}

#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, _args: DownloadWorkerArgs) -> Result<()> {
        // TODO: Some actual work goes here...

        Ok(())
    }
}
