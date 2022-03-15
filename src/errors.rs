//! Application specific error codes and messages

use many::define_application_many_error;

define_application_many_error!({
    5000: pub fn scheduler_error(e) => "Tokio cron scheduler error {e}",
    5001: pub fn job_error(e) => "Tokio job error {e}",
    5002: pub fn job_scheduling_error(e) => "Tokio cron scheduler job scheduling error {e}",
    5003: pub fn ledger_send_error(e) => "Ledger send error {e}",
    5004: pub fn storage_mutex_error(e) => "Storage mutex error {e}",
    5005: pub fn storage_error(e) => "Storage error {e}",
    5006: pub fn response_deserialization_error(e) => "Response deserialization error {e}",
});
