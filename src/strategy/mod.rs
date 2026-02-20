pub mod momentum;

use crate::domain::TradeIntent;

pub trait StrategyEngine {
    fn name(&self) -> &'static str;
    fn tick(&mut self) -> anyhow::Result<Vec<TradeIntent>>;
}
