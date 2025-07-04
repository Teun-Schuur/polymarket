use crate::{SimpleOrder};


#[inline]
pub fn get_midpoint(bid: f64, ask: f64) -> f64 {
    if bid <= 0.0 || ask <= 0.0 {
        return 0.0;
    }
    (bid + ask) / 2.0
}

pub fn get_midpoint_from_slices(bid: &[SimpleOrder], ask: &[SimpleOrder]) -> f64 {
    let best_bid = bid.first().map_or(0.0, |b| b.price);
    let best_ask = ask.first().map_or(0.0, |a| a.price);
    get_midpoint(best_bid, best_ask)
}


#[inline]
pub fn get_spread(bid: f64, ask: f64) -> f64 {
    if bid <= 0.0 || ask <= 0.0 {
        return 0.0;
    }
    ask - bid
}

pub fn get_spread_from_slices(bid: &[SimpleOrder], ask: &[SimpleOrder]) -> f64 {
    let best_bid = bid.first().map_or(0.0, |b| b.price);
    let best_ask = ask.first().map_or(0.0, |a| a.price);
    get_spread(best_bid, best_ask)
}
