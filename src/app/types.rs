//! Type definitions for the application

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SelectedTab {
    #[default]
    Orderbook,
    PriceHistory,
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    pub fn previous(self) -> Self {
        match self {
            Self::Orderbook => Self::PriceHistory,
            Self::PriceHistory => Self::Orderbook,
        }
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub fn next(self) -> Self {
        match self {
            Self::Orderbook => Self::PriceHistory,
            Self::PriceHistory => Self::Orderbook,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum MarketSelectorTab {
    #[default]
    AllMarkets,
    Events,
    Strategies,
}

impl MarketSelectorTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    pub fn previous(self) -> Self {
        match self {
            Self::AllMarkets => Self::Strategies,
            Self::Events => Self::AllMarkets,
            Self::Strategies => Self::Events,
        }
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub fn next(self) -> Self {
        match self {
            Self::AllMarkets => Self::Events,
            Self::Events => Self::Strategies,
            Self::Strategies => Self::AllMarkets,
        }
    }
}
