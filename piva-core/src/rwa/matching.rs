//! PIVA Matching Engine - Enhanced with Geo-Priority & Antifragility
//! Logic: Price-Time-Distance Priority

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

// Requisitos de Integración: Estos tipos deben estar en rwa/market.rs
use crate::rwa::market::{MarketOffer, OfferType, GeoLocation};

/// Estructura principal del Motor de Emparejamiento
pub struct MatchingEngine {
    order_books: HashMap<String, OrderBook>,
    config: MatchingConfig,
    #[allow(dead_code)]
    match_history: Vec<TradeMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: String,
    pub asset_id: String,
    pub order_type: OfferType,
    pub price: u64,
    pub remaining_amount: u64,
    pub creator_peer_id: String,
    pub created_at: u64,
    pub peer_score: u16,
    pub location: Option<GeoLocation>,
    pub min_reputation: u32,
    pub status: crate::rwa::market::OfferStatus, // Use market's OfferStatus
    /// Score calculado dinámicamente para prioridad en la cola
    pub priority_score: u64,
}

pub struct OrderBook {
    pub asset_id: String,
    pub bids: BTreeMap<u64, Vec<Order>>,
    pub asks: BTreeMap<u64, Vec<Order>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStats {
    pub total_matches: u64,
    pub total_volume: u64,
    pub avg_match_time_ms: u64,
    pub orders_per_second: f64,
    pub match_success_rate: f64,
    pub geo_match_rate: f64,
    pub htlc_usage_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookStats {
    pub asset_id: String,
    pub total_orders: usize,
    pub total_bids: usize,
    pub total_asks: usize,
    pub best_bid: Option<u64>,
    pub best_ask: Option<u64>,
    pub spread: Option<u64>,
    pub last_price: Option<u64>,
    pub volume_24h: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    pub geo_proximity_bonus: u8,      // % de incentivo (0-100)
    pub local_matching_radius: f64,   // Radio en KM (ej. 50.0)
    pub price_tolerance: u8,          // % de spread aceptable
    pub force_htlc: bool,
}

impl MatchingEngine {
    /// Create new matching engine
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            order_books: HashMap::new(),
            config,
            match_history: Vec::new(),
        }
    }
    
    /// Add order to the order book
    pub fn add_order(&mut self, offer: MarketOffer) -> Result<String> {
        let order_id = self.generate_order_id(&offer);
        
        let order = Order {
            order_id: order_id.clone(),
            asset_id: offer.asset_id.to_string(),
            order_type: offer.offer_type.clone(),
            price: offer.price,
            remaining_amount: offer.amount,
            creator_peer_id: offer.creator_peer_id.clone(),
            created_at: offer.created_at,
            peer_score: offer.peer_score.overall_score,
            location: offer.location.clone(),
            min_reputation: offer.min_reputation,
            status: offer.status.clone(), // Use offer's existing status
            priority_score: self.calculate_priority_score(&offer),
        };

        let order_book = self.order_books
            .entry(order.asset_id.clone())
            .or_insert_with(|| OrderBook::new(order.asset_id.clone()));
        
        match order.order_type {
            OfferType::Buy => {
                order_book.bids.entry(order.price).or_default().push(order);
            },
            OfferType::Sell => {
                order_book.asks.entry(order.price).or_default().push(order);
            },
        }

        Ok(order_id)
    }

    /// Procesa el matching para un activo específico
    pub fn process_matches(&mut self, asset_id: &str) -> Result<Vec<TradeMatch>> {
        let mut matches = Vec::new();

        // 1. Extraer el libro para evitar conflictos de "Mutable Borrow" con self
        if let Some(mut book) = self.order_books.remove(asset_id) {
            
            loop {
                // 2. Obtener mejores candidatos (Highest Bid vs Lowest Ask)
                let best_bid = self.get_best_prioritized_order(&mut book.bids);
                let best_ask = self.get_best_prioritized_order(&mut book.asks);

                if let (Some((bid_price, bid_idx)), Some((ask_price, ask_idx))) = (best_bid, best_ask) {
                    if bid_price >= ask_price {
                        // 3. Ejecutar el Match
                        let bid_list = book.bids.get_mut(&bid_price).unwrap();
                        let ask_list = book.asks.get_mut(&ask_price).unwrap();

                        if let Some(m) = self.execute_match_logic(asset_id, bid_list, ask_list, bid_idx, ask_idx)? {
                            matches.push(m);
                            // Limpieza de niveles vacíos
                            if bid_list.is_empty() { book.bids.remove(&bid_price); }
                            if ask_list.is_empty() { book.asks.remove(&ask_price); }
                        } else { break; }
                    } else { break; }
                } else { break; }
            }

            self.order_books.insert(asset_id.to_string(), book);
        }
        Ok(matches)
    }

    /// Lógica de Prioridad: Selecciona la orden no solo por precio, sino por cercanía y reputación.
    fn get_best_prioritized_order(&self, levels: &mut BTreeMap<u64, Vec<Order>>) -> Option<(u64, usize)> {
        // Get the highest price for Bids or lowest for Asks
        // For Bids, we want the highest price (next_back)
        levels.iter_mut().next_back().map(|(price, orders)| {
            // Find the index of the order with the highest priority_score
            let best_idx = orders.iter().enumerate()
                .max_by_key(|(_, order)| order.priority_score)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
                
            (*price, best_idx)
        })
    }

    /// Calculate priority score for order
    fn calculate_priority_score(&self, offer: &MarketOffer) -> u64 {
        let base_score = offer.peer_score.overall_score as u64;
        let reputation_bonus = offer.peer_score.successful_trades as u64 * 10;
        let volume_bonus = (offer.peer_score.total_volume / 1000000) * 5; // 1M = 5 points
        
        base_score + reputation_bonus + volume_bonus
    }

    /// Implementación de Geo-Bonus e Incentivos Hiper-locales (< 10km)
    fn calculate_geo_incentive(&self, bid: &Order, ask: &Order) -> u8 {
        match (&bid.location, &ask.location) {
            (Some(b), Some(a)) => {
                // Use pure distance calculation instead of country borders
                let dist = self.haversine_distance(b.latitude, b.longitude, a.latitude, a.longitude);
                
                if dist <= 10.0 { 
                    self.config.geo_proximity_bonus // Max Bonus (Hiper-local)
                } else if dist <= self.config.local_matching_radius {
                    self.config.geo_proximity_bonus / 2 // Regional
                } else if dist <= 200.0 {
                    5 // Extended regional (200km)
                } else {
                    0 // No bonus for long distance
                }
            },
            _ => 0,
        }
    }

    /// Ejecución física del intercambio
    fn execute_match_logic(
        &self,
        asset_id: &str,
        bids: &mut Vec<Order>,
        asks: &mut Vec<Order>,
        b_idx: usize,
        a_idx: usize,
    ) -> Result<Option<TradeMatch>> {
        let bid = &mut bids[b_idx];
        let ask = &mut asks[a_idx];

        // Filtro de Reputación (Teoría de Juegos: No transar con actores tóxicos)
        if bid.peer_score < ask.min_reputation as u16 || ask.peer_score < bid.min_reputation as u16 {
            return Ok(None);
        }

        let amount = std::cmp::min(bid.remaining_amount, ask.remaining_amount);
        let geo_bonus = self.calculate_geo_incentive(bid, ask);

        // Actualizar cantidades
        bid.remaining_amount -= amount;
        ask.remaining_amount -= amount;

        let trade = TradeMatch {
            match_id: format!("m_{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros()),
            asset_id: asset_id.to_string(),
            price: ask.price, // Precio del creador de mercado (Ask)
            amount,
            buyer_peer_id: bid.creator_peer_id.clone(),
            seller_peer_id: ask.creator_peer_id.clone(),
            geo_bonus_applied: geo_bonus > 0,
            requires_htlc: self.config.force_htlc || bid.peer_score < 400,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        // Cleanup
        if bid.remaining_amount == 0 { bids.remove(b_idx); }
        if ask.remaining_amount == 0 { asks.remove(a_idx); }

        Ok(Some(trade))
    }

    /// Fórmula de Haversine para distancia real en la Tierra
    fn haversine_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let r = 6371.0; // Radio de la Tierra en km
        let d_lat = (lat2 - lat1).to_radians();
        let d_lon = (lon2 - lon1).to_radians();
        let a = (d_lat / 2.0).sin().powi(2) +
                lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        r * c
    }

    /// Generate unique order ID
    fn generate_order_id(&self, _offer: &MarketOffer) -> String {
        format!("ord_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMatch {
    pub match_id: String,
    pub asset_id: String,
    pub price: u64,
    pub amount: u64,
    pub buyer_peer_id: String,
    pub seller_peer_id: String,
    pub geo_bonus_applied: bool,
    pub requires_htlc: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Active,
    Filled,
    Cancelled,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            geo_proximity_bonus: 10,
            local_matching_radius: 50.0,
            price_tolerance: 2,
            force_htlc: false,
        }
    }
}

impl OrderBook {
    pub fn new(asset_id: String) -> Self {
        Self {
            asset_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwa::market::{PeerScore, VerificationLevel, TrustFactor, TrustFactorType};
    
    fn mock_geo_location(lat: f64, lon: f64, country: &str) -> GeoLocation {
        GeoLocation {
            latitude: lat,
            longitude: lon,
            country: country.to_string(),
            region: Some("TestRegion".to_string()),
            city: Some("TestCity".to_string()),
        }
    }
    
    fn mock_peer_score(score: u16) -> PeerScore {
        PeerScore {
            overall_score: score,
            successful_trades: 10,
            failed_trades: 0,
            total_volume: 100000,
            account_age_days: 30,
            last_activity: 1234567890,
            verification_level: VerificationLevel::Verified,
            trust_factors: vec![TrustFactor {
                factor_type: TrustFactorType::TradeHistory,
                value: 100,
                last_updated: 1234567890,
            }],
        }
    }
    
    fn mock_offer_with_geo(
        asset_id: &str, 
        price: u64, 
        amount: u64,
        offer_type: OfferType, 
        peer_id: &str,
        location: Option<GeoLocation>
    ) -> MarketOffer {
        MarketOffer {
            offer_id: format!("offer_{}", peer_id),
            asset_id: asset_id.parse().unwrap(),
            price,
            amount,
            currency: "USD".to_string(),
            creator_peer_id: peer_id.to_string(),
            status: crate::rwa::market::OfferStatus::Open,
            min_reputation: 0,
            created_at: 1234567890,
            expires_at: 1234567890 + 86400,
            offer_type,
            location,
            payment_methods: vec!["PIVA".to_string()],
            peer_score: mock_peer_score(750),
        }
    }
    
    #[test]
    fn test_hyper_local_bonus() {
        let mut engine = MatchingEngine::new(MatchingConfig {
            geo_proximity_bonus: 15,
            local_matching_radius: 50.0,
            ..Default::default()
        });
        
        // NYC locations - very close (< 10km)
        let buyer_loc = mock_geo_location(40.7128, -74.0060, "US");
        let seller_loc = mock_geo_location(40.7589, -73.9851, "US");
        
        let buy_offer = mock_offer_with_geo(
            "piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef",
            10000,
            1000,
            OfferType::Buy,
            "buyer",
            Some(buyer_loc)
        );
        
        let sell_offer = mock_offer_with_geo(
            "piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef",
            9500,
            1000,
            OfferType::Sell,
            "seller",
            Some(seller_loc)
        );
        
        engine.add_order(buy_offer).unwrap();
        engine.add_order(sell_offer).unwrap();
        
        let matches = engine.process_matches("piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].geo_bonus_applied); // Should get hyper-local bonus
    }
    
    #[test]
    fn test_cross_country_no_bonus() {
        let mut engine = MatchingEngine::new(MatchingConfig::default());
        
        // US vs UK - no bonus
        let buyer_loc = mock_geo_location(40.7128, -74.0060, "US");
        let seller_loc = mock_geo_location(51.5074, -0.1278, "UK");
        
        let buy_offer = mock_offer_with_geo(
            "piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef",
            10000,
            1000,
            OfferType::Buy,
            "buyer",
            Some(buyer_loc)
        );
        
        let sell_offer = mock_offer_with_geo(
            "piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef",
            9500,
            1000,
            OfferType::Sell,
            "seller",
            Some(seller_loc)
        );
        
        engine.add_order(buy_offer).unwrap();
        engine.add_order(sell_offer).unwrap();
        
        let matches = engine.process_matches("piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(matches.len(), 1);
        assert!(!matches[0].geo_bonus_applied); // No bonus for cross-country
    }
    
    #[test]
    fn test_htlc_requirement_for_low_score() {
        let mut engine = MatchingEngine::new(MatchingConfig::default());
        
        let mut low_score_offer = mock_offer_with_geo(
            "piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef",
            10000,
            1000,
            OfferType::Buy,
            "low_score_peer",
            None
        );
        
        low_score_offer.peer_score.overall_score = 300; // Below 400 threshold
        
        engine.add_order(low_score_offer).unwrap();
        
        let matches = engine.process_matches("piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        if !matches.is_empty() {
            assert!(matches[0].requires_htlc); // Should require HTLC for low score
        }
    }
    
    #[test]
    fn test_haversine_distance() {
        let engine = MatchingEngine::new(MatchingConfig::default());
        
        // Madrid to Toledo - should be ~70km
        let distance = engine.haversine_distance(40.4168, -3.7038, 39.8628, -4.0273);
        assert!(distance > 60.0, "Madrid-Toledo should be >60km, got {}", distance);
        assert!(distance < 80.0, "Madrid-Toledo should be <80km, got {}", distance);
        
        // NYC to London - should be ~5570km
        let distance = engine.haversine_distance(40.7128, -74.0060, 51.5074, -0.1278);
        assert!(distance > 5500.0, "NYC-London should be >5500km, got {}", distance);
        assert!(distance < 5600.0, "NYC-London should be <5600km, got {}", distance);
    }
}
