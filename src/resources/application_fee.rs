use params::{List, Timestamp};
use resources::{Currency, Refund};

/// The resource representing a Stripe application fee.
///
/// For more details see https://stripe.com/docs/api#application_fees.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApplicationFee {
    pub id: String,
    pub object: String,
    pub account: String,
    pub amount: u64,
    pub amount_refunded: i64,
    pub application: String,
    pub balance_transaction: String,
    pub charge: String,
    pub created: Timestamp,
    pub currency: Currency,
    pub livemode: bool,
    pub originating_transaction: Option<String>,
    pub refunded: bool,
    pub refunds: List<Refund>,
}
