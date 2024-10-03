use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;

#[derive(Debug, Default, Clone)]
pub struct TicketPrice {
    pub sequences_count: u32,
    pub price_per_ticket: FPUSDC,
}

#[async_trait]
pub trait TicketPriceCalculator: Sync + Send {
    async fn calculate(&self, balance: FPUSDC) -> TicketPrice;
    async fn price(&self) -> FPUSDC;
}

#[derive(Debug, Clone)]
pub struct ConstantTicketPriceCalculator {
    price: FPUSDC,
}

impl ConstantTicketPriceCalculator {
    pub fn new(price: FPUSDC) -> Self {
        Self { price }
    }
}

#[async_trait]
impl TicketPriceCalculator for ConstantTicketPriceCalculator {
    async fn calculate(&self, balance: FPUSDC) -> TicketPrice {
        let sequences_count = balance.checked_div(self.price).unwrap().as_whole_number();
        let price_per_ticket = self.price;
        TicketPrice {
            sequences_count: sequences_count as _,
            price_per_ticket,
        }
    }

    async fn price(&self) -> FPUSDC {
        self.price
    }
}
