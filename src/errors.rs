use many::define_application_many_error;

define_application_many_error!({
    5000: pub fn scheduler_error(e) => "Tokio cron scheduler error {e}",
    5001: pub fn job_error(e) => "Tokio job error {e}",
    5002: pub fn job_scheduling_error(e) => "Tokio cron scheduler job scheduling error {e}",
    5003: pub fn ledger_send_error(e) => "Ledger send error {e}"
});
