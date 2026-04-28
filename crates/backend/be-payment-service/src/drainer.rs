use std::sync::Arc;

use be_remote_db::{ClaimedProvisioningJob, DatabaseManager, DbError};
use chrono::Duration as ChronoDuration;
use tokio::sync::oneshot;
use tokio::time::{Duration, sleep};

use crate::analytics;
use crate::provision::{ProvisionError, StripeBillingProvisioner};

/// Maximum number of jobs claimed per tick. Sized so a backlog drains in a
/// few iterations without monopolising the Stripe API.
const BATCH_SIZE: usize = 50;

/// How long a claimed job stays "leased" before another worker may retry it.
/// Sized to be longer than the worst-case Stripe + DB round-trip.
const LEASE: ChronoDuration = ChronoDuration::minutes(2);

/// Time between drainer ticks when the queue is empty.
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(15);

/// Time between drainer ticks when there might be more work to do (we hit the
/// batch size). Lets a backlog drain quickly without busy-looping.
const BUSY_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Cap on transient retry backoff.
const MAX_BACKOFF: ChronoDuration = ChronoDuration::hours(1);

pub struct DrainerHandle {
    shutdown: oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}

impl DrainerHandle {
    pub async fn shutdown(self) {
        let Self { shutdown, join } = self;
        let _ = shutdown.send(());
        let _ = join.await;
    }
}

/// Spawn the background worker that drains the
/// `stripe.customer_provisioning_jobs` outbox.
///
/// Each job is leased via `claim_due_provisioning_jobs`, then the provisioner
/// is called and either the row is deleted as part of the link-write
/// transaction (success), rescheduled with backoff (transient failure), or
/// dead-lettered (permanent failure).
pub fn spawn_drainer(
    db: Arc<DatabaseManager>,
    provisioner: Arc<StripeBillingProvisioner>,
) -> DrainerHandle {
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let join = tokio::spawn(async move {
        tracing::info!("Stripe customer provisioning drainer started");
        loop {
            let processed = match tick(&db, &provisioner).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!(error = %e, "Drainer tick failed");
                    0
                }
            };

            let next_delay = if processed >= BATCH_SIZE {
                BUSY_POLL_INTERVAL
            } else {
                IDLE_POLL_INTERVAL
            };

            tokio::select! {
                _ = sleep(next_delay) => {}
                _ = &mut shutdown_rx => {
                    tracing::info!("Stripe customer provisioning drainer shutting down");
                    break;
                }
            }
        }
    });

    DrainerHandle {
        shutdown: shutdown_tx,
        join,
    }
}

async fn tick(
    db: &DatabaseManager,
    provisioner: &StripeBillingProvisioner,
) -> Result<usize, DbError> {
    let jobs = db
        .claim_due_provisioning_jobs()
        .limit(BATCH_SIZE as i64)
        .lease(LEASE)
        .call()
        .await?;

    let count = jobs.len();
    if count > 0 {
        tracing::debug!(claimed = count, "Drainer claimed jobs");
    }

    for job in jobs {
        process_job(db, provisioner, job).await;
    }

    Ok(count)
}

async fn process_job(
    db: &DatabaseManager,
    provisioner: &StripeBillingProvisioner,
    job: ClaimedProvisioningJob,
) {
    match provisioner.ensure_customer(job.user_id, &job.email).await {
        Ok(_) => {
            // persist_link inside the provisioner has already deleted the
            // job row in the same transaction as the link write — nothing
            // more to do here.
            analytics::track_customer_provisioning_attempt("succeeded");
        }
        Err(err) => {
            analytics::track_customer_provisioning_attempt(err.kind());
            let next_attempts = job.attempts.saturating_add(1);
            let permanent = matches!(err, ProvisionError::Permanent(_));

            if permanent {
                tracing::error!(
                    user_id = %job.user_id,
                    attempts = next_attempts,
                    error = %err,
                    "Stripe customer provisioning permanently failed — dead-lettering"
                );
                if let Err(e) = db
                    .dead_letter_stripe_customer_provisioning(job.user_id, &err.to_string())
                    .await
                {
                    tracing::error!(
                        user_id = %job.user_id,
                        error = %e,
                        "Failed to dead-letter provisioning job"
                    );
                }
            } else {
                tracing::warn!(
                    user_id = %job.user_id,
                    attempts = next_attempts,
                    error = %err,
                    "Stripe customer provisioning failed — will retry"
                );
                let backoff = exponential_backoff(next_attempts);
                if let Err(e) = db
                    .fail_stripe_customer_provisioning(job.user_id, &err.to_string(), backoff)
                    .await
                {
                    tracing::error!(
                        user_id = %job.user_id,
                        error = %e,
                        "Failed to record provisioning failure"
                    );
                }
            }
        }
    }
}

fn exponential_backoff(attempts: i32) -> ChronoDuration {
    let exp = attempts.clamp(1, 12) as u32;
    let secs = 2_i64.saturating_pow(exp);
    ChronoDuration::seconds(secs).min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_then_caps() {
        assert_eq!(exponential_backoff(1), ChronoDuration::seconds(2));
        assert_eq!(exponential_backoff(5), ChronoDuration::seconds(32));
        assert_eq!(exponential_backoff(11), ChronoDuration::seconds(2048));
        assert_eq!(exponential_backoff(12), MAX_BACKOFF);
        assert_eq!(exponential_backoff(20), MAX_BACKOFF);
    }
}
