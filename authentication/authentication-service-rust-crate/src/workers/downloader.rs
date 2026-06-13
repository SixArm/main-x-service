//! Loco background-worker scaffold. Currently a no-op stub kept as the
//! template for future async jobs; `perform` does nothing yet.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Background worker stub carrying the loco [`AppContext`].
pub struct DownloadWorker {
    /// Shared loco application context (db, config, mailer, …).
    pub ctx: AppContext,
}

/// Arguments for a [`DownloadWorker`] job.
#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    /// Target user's global id.
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
