#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Trade Date <= Value Date <= Delivery Date failed")]
    InvalidDates,
    #[error("Trade contained a cancel witness")]
    IsCanceled,
    #[error("Trade lacks an valid approved witness")]
    NoApproved,
    #[error("Update witness was found, but no subsequent 'approve'")]
    PendingApproval,
    #[error("Trade is missing a submit witness")]
    MissingSubmit,
    #[error("Trade has already been executed and booked")]
    AlreadyExecuted,
}
