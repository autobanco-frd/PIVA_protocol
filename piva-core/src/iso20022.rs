//! ISO 20022 XML Generator Module
//! 
//! Implements ISO 20022 financial message generation for PIVA protocol.
//! This enables PIVA transactions to be compatible with global banking systems.

use serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::rwa::market::{MarketOffer, PaymentMethod, GeoLocation};

/// ISO 20022 Message Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsoMessageType {
    /// Customer Credit Transfer (pain.001)
    CustomerCreditTransfer,
    /// Customer Payment Status (pain.002)
    CustomerPaymentStatus,
    /// Financial Institution Transfer (pacs.008)
    FinancialInstitutionTransfer,
    /// Payment Return (pacs.004)
    PaymentReturn,
}

/// ISO 20022 Message Generator
pub struct Iso20022Generator {
    /// Sending institution identifier
    pub sending_institution: String,
    
    /// BIC code
    pub bic: String,
    
    /// Default currency
    pub default_currency: String,
}

impl Iso20022Generator {
    /// Create new ISO 20022 generator
    pub fn new(sending_institution: String, bic: String, default_currency: String) -> Self {
        Self {
            sending_institution,
            bic,
            default_currency,
        }
    }
    
    /// Generate pain.001 Customer Credit Transfer message
    pub fn generate_pain_001(
        &self,
        offer: &MarketOffer,
        debtor_account: &str,
        creditor_account: &str,
        amount: u64,
        payment_details: &str,
        execution_date: Option<DateTime<Utc>>,
    ) -> Result<String> {
        let message_id = self.generate_message_id();
        let creation_time = Utc::now();
        let requested_date = execution_date.unwrap_or_else(|| creation_time + chrono::Duration::hours(24));
        
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.03">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>{message_id}</MsgId>
      <CreDtTm>{creation_time}</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{amount}.00</CtrlSum>
      <InitgPty>
        <Nm>{sending_institution}</Nm>
        <Id>
          <OrgId>
            <BIC>{bic}</BIC>
          </OrgId>
        </Id>
      </InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-{message_id}</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{amount}.00</CtrlSum>
      <ReqdExctnDt>{requested_date}</ReqdExctnDt>
      <Dbtr>
        <Nm>PIVA Node {node_id}</Nm>
        <PstlAdr>
          <Ctry>{country}</Ctry>
          <AdrLine>{address}</AdrLine>
        </PstlAdr>
      </Dbtr>
      <DbtrAcct>
        <Id>
          <IBAN>{debtor_account}</IBAN>
        </Id>
        <Ccy>{currency}</Ccy>
      </DbtrAcct>
      <CdtrAgt>
        <FinInstnId>
          <BIC>{creditor_bic}</BIC>
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{creditor_name}</Nm>
        <PstlAdr>
          <Ctry>{creditor_country}</Ctry>
          <AdrLine>{creditor_address}</AdrLine>
        </PstlAdr>
      </Cdtr>
      <CdtrAcct>
        <Id>
          <IBAN>{creditor_account}</IBAN>
        </Id>
        <Ccy>{currency}</Ccy>
      </CdtrAcct>
      <ChrgBr>
        <Amt>{fee_amount}.00</Amt>
        <Ccy>{currency}</Ccy>
      </ChrgBr>
      <CdtTrfTxInf>
        <PmtId>
          <InstrId>INSTR-{message_id}</InstrId>
          <EndToEndId>E2E-{message_id}</EndToEndId>
        </PmtId>
        <Amt>
          <InstdAmt Ccy="{currency}">{amount}.00</InstdAmt>
        </Amt>
        <ChrgBr>SHAR</ChrgBr>
        <CdtrAgt>
          <FinInstnId>
            <BIC>{creditor_bic}</BIC>
          </FinInstnId>
        </CdtrAgt>
        <Cdtr>
          <Nm>{creditor_name}</Nm>
        </Cdtr>
        <CdtrAcct>
          <Id>
            <IBAN>{creditor_account}</IBAN>
          </Id>
        </CdtrAcct>
        <RmtInf>
          <Ustrd>{payment_details}</Ustrd>
        </RmtInf>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#,
            message_id = message_id,
            creation_time = creation_time.format("%Y-%m-%dT%H:%M:%S"),
            amount = amount,
            sending_institution = self.sending_institution,
            bic = self.bic,
            node_id = offer.creator_node_id,
            country = self.get_country_code(&offer.location),
            address = self.get_address(&offer.location),
            debtor_account = debtor_account,
            currency = offer.currency,
            creditor_bic = self.get_creditor_bic(&offer.payment_methods),
            creditor_name = self.get_creditor_name(&offer.metadata),
            creditor_country = self.get_creditor_country(&offer.location),
            creditor_address = self.get_creditor_address(&offer.location),
            creditor_account = creditor_account,
            fee_amount = self.calculate_fee(amount),
            requested_date = requested_date.format("%Y-%m-%d"),
            creditor_bic = self.get_creditor_bic(&offer.payment_methods),
            creditor_name = self.get_creditor_name(&offer.metadata),
            payment_details = payment_details
        );
        
        Ok(xml)
    }
    
    /// Generate pacs.008 Financial Institution Transfer message
    pub fn generate_pacs_008(
        &self,
        original_message_id: &str,
        offer: &MarketOffer,
        debtor_account: &str,
        creditor_account: &str,
        amount: u64,
        status: TransferStatus,
    ) -> Result<String> {
        let message_id = self.generate_message_id();
        let creation_time = Utc::now();
        
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.02">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>{message_id}</MsgId>
      <CreDtTm>{creation_time}</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{amount}.00</CtrlSum>
      <InitgPty>
        <Nm>{sending_institution}</Nm>
        <Id>
          <OrgId>
            <BIC>{bic}</BIC>
          </OrgId>
        </Id>
      </InitgPty>
    </GrpHdr>
    <CdtTrfTxInf>
      <TxId>{tx_id}</TxId>
      <Amt>
        <InstdAmt Ccy="{currency}">{amount}.00</InstdAmt>
      </Amt>
      <CdtrAgt>
        <FinInstnId>
          <BIC>{creditor_bic}</BIC>
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{creditor_name}</Nm>
      </Cdtr>
      <CdtrAcct>
        <Id>
          <IBAN>{creditor_account}</IBAN>
        </Id>
      </CdtrAcct>
      <DbtrAgt>
        <FinInstnId>
          <BIC>{debtor_bic}</BIC>
        </FinInstnId>
      </DbtrAgt>
      <Dbtr>
        <Nm>{debtor_name}</Nm>
      </Dbtr>
      <DbtrAcct>
        <Id>
          <IBAN>{debtor_account}</IBAN>
        </Id>
      </DbtrAcct>
      <RmtInf>
        <Ustrd>{payment_details}</Ustrd>
      </RmtInf>
      {status_block}
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#,
            message_id = message_id,
            creation_time = creation_time.format("%Y-%m-%dT%H:%M:%S"),
            amount = amount,
            sending_institution = self.sending_institution,
            bic = self.bic,
            tx_id = format!("TX-{}", message_id),
            currency = offer.currency,
            creditor_bic = self.get_creditor_bic(&offer.payment_methods),
            creditor_name = self.get_creditor_name(&offer.metadata),
            creditor_account = creditor_account,
            debtor_bic = self.bic,
            debtor_name = format!("PIVA Node {}", offer.creator_node_id),
            debtor_account = debtor_account,
            payment_details = format!("PIVA Asset Trade: {}", offer.asset_id),
            status_block = self.generate_status_block(status)
        );
        
        Ok(xml)
    }
    
    /// Generate message ID
    fn generate_message_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        hasher.write(Utc::now().timestamp().to_string().as_bytes());
        hasher.write(self.sending_institution.as_bytes());
        format!("PIVA-{:x}", hasher.finish())
    }
    
    /// Get country code from location
    fn get_country_code(&self, location: &Option<GeoLocation>) -> String {
        location.as_ref()
            .map(|loc| loc.country.clone())
            .unwrap_or_else(|| "US".to_string())
    }
    
    /// Get address from location
    fn get_address(&self, location: &Option<GeoLocation>) -> String {
        location.as_ref().map_or("Unknown".to_string(), |loc| {
            format!("{}, {}", 
                loc.city.as_deref().unwrap_or("Unknown"),
                loc.region.as_deref().unwrap_or("Unknown")
            )
        })
    }
    
    /// Get creditor BIC from payment methods
    fn get_creditor_bic(&self, payment_methods: &[PaymentMethod]) -> String {
        payment_methods.iter()
            .find(|pm| matches!(pm, PaymentMethod::BankTransfer))
            .map_or("UNKNOWNBICXXX".to_string(), |_| "BANKDEFFXXX".to_string())
    }
    
    /// Get creditor name from metadata
    fn get_creditor_name(&self, metadata: &crate::rwa::market::OfferMetadata) -> String {
        metadata.contact.as_ref()
            .map(|contact| contact.identifier.clone())
            .unwrap_or_else(|| "Unknown Creditor".to_string())
    }
    
    /// Get creditor country from location
    fn get_creditor_country(&self, location: &Option<GeoLocation>) -> String {
        self.get_country_code(location)
    }
    
    /// Get creditor address from location
    fn get_creditor_address(&self, location: &Option<GeoLocation>) -> String {
        self.get_address(location)
    }
    
    /// Calculate fee (0.5% minimum)
    fn calculate_fee(&self, amount: u64) -> u64 {
        std::cmp::max(amount * 5 / 1000, 100) // 0.5% minimum 100 units
    }
    
    /// Generate status block for pacs.008
    fn generate_status_block(&self, status: TransferStatus) -> String {
        match status {
            TransferStatus::Accepted => format!(
                r#"<StsRsnInf>
                  <Cd>{}</Cd>
                  <AddtlInf>Payment accepted</AddtlInf>
                </StsRsnInf>"#,
                "ACCP"
            ),
            TransferStatus::Rejected => format!(
                r#"<StsRsnInf>
                  <Cd>{}</Cd>
                  <AddtlInf>Payment rejected</AddtlInf>
                </StsRsnInf>"#,
                "RJCT"
            ),
            TransferStatus::Pending => format!(
                r#"<StsRsnInf>
                  <Cd>{}</Cd>
                  <AddtlInf>Payment pending</AddtlInf>
                </StsRsnInf>"#,
                "PDNG"
            ),
        }
    }
}

/// Transfer Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Payment accepted
    Accepted,
    /// Payment rejected
    Rejected,
    /// Payment pending
    Pending,
}

/// ISO 20022 Validator
pub struct IsoValidator;

impl IsoValidator {
    /// Validate ISO 20022 message structure
    pub fn validate_message(xml: &str, message_type: IsoMessageType) -> Result<bool> {
        // Basic XML structure validation
        if !xml.starts_with("<?xml") || !xml.contains("<Document") {
            return Ok(false);
        }
        
        // Validate specific message type structure
        match message_type {
            IsoMessageType::CustomerCreditTransfer => {
                xml.contains("CstmrCdtTrfInitn") && 
                xml.contains("GrpHdr") && 
                xml.contains("PmtInf")
            },
            IsoMessageType::FinancialInstitutionTransfer => {
                xml.contains("FIToFICstmrCdtTrf") && 
                xml.contains("GrpHdr") && 
                xml.contains("CdtTrfTxInf")
            },
            _ => false,
        };
        
        Ok(true)
    }
    
    /// Extract message ID from XML
    pub fn extract_message_id(xml: &str) -> Option<String> {
        use regex::Regex;
        
        let msg_id_regex = Regex::new(r"<MsgId>([^<]+)</MsgId>").ok()?;
        msg_id_regex.captures(xml).map(|caps| caps[1].to_string())
    }
    
    /// Extract amount from XML
    pub fn extract_amount(xml: &str) -> Option<u64> {
        use regex::Regex;
        
        let amount_regex = Regex::new(r"<InstdAmt[^>]*>([^<]+)</InstdAmt>").ok()?;
        let amount_str = amount_regex.captures(xml)?.get(1)?.as_str();
        
        // Remove decimal point and convert
        amount_str.replace('.', "").parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwa::market::{OfferMetadata, PeerScore, VerificationLevel};
    
    #[test]
    fn test_pain_001_generation() {
        let generator = Iso20022Generator::new(
            "PIVA Bank".to_string(),
            "PIVABANKXXX".to_string(),
            "USD".to_string()
        );
        
        let offer = MarketOffer {
            offer_id: "test_offer_123".to_string(),
            asset_id: "test_asset_456".parse().unwrap(),
            price: 100000, // $1000.00
            currency: "USD".to_string(),
            expiration: 1234567890,
            creator_node_id: "node_abc123".to_string(),
            peer_score: PeerScore {
                overall_score: 800,
                successful_trades: 50,
                failed_trades: 2,
                total_volume: 5000000,
                account_age_days: 365,
                last_activity: 1234567890,
                verification_level: VerificationLevel::Trusted,
                trust_factors: vec![],
            },
            offer_type: crate::rwa::market::OfferType::Sell,
            min_amount: 10000,
            max_amount: 100000,
            location: Some(GeoLocation {
                country: "US".to_string(),
                region: Some("CA".to_string()),
                city: Some("San Francisco".to_string()),
                latitude: Some(37.7749),
                longitude: Some(-122.4194),
                radius_km: Some(50),
            }),
            payment_methods: vec![PaymentMethod::BankTransfer],
            metadata: OfferMetadata {
                description: Some("Test offer".to_string()),
                terms: Some("Payment within 24 hours".to_string()),
                contact: Some(ContactInfo {
                    preferred_method: PaymentMethod::BankTransfer,
                    identifier: "test@example.com".to_string(),
                    availability: None,
                }),
                tags: vec!["test".to_string()],
                languages: vec!["en".to_string()],
            },
            created_at: 1234567890,
            network: "devnet".to_string(),
        };
        
        let xml = generator.generate_pain_001(
            &offer,
            "US12345678901234567890",
            "DE98765432109876543210",
            50000, // $500.00
            "PIVA Asset Purchase: test_asset_456",
            None,
        ).unwrap();
        
        assert!(xml.contains("CstmrCdtTrfInitn"));
        assert!(xml.contains("50000.00"));
        assert!(xml.contains("US12345678901234567890"));
        assert!(xml.contains("DE98765432109876543210"));
        assert!(xml.contains("PIVA Asset Purchase"));
    }
}
