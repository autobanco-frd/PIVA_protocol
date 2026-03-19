//! ISO 20022 XML Generator Module
//! 
//! Implements ISO 20022 financial message generation for PIVA protocol.
//! This enables PIVA transactions to be compatible with global banking systems.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Iso20022Report {
    /// Message ID
    pub msg_id: String,
    
    /// Creation date and time
    pub creation_date_time: DateTime<Utc>,
    
    /// Settlement amount
    pub settlement_amount: f64,
    
    /// Currency code
    pub currency: String,
    
    /// Creditor name
    pub creditor_name: String,
    
    /// Debtor name
    pub debtor_name: String,
    
    /// Transaction description
    pub description: String,
    
    /// Payment method
    pub payment_method: String,
}

impl Iso20022Report {
    /// Create new ISO 20022 report
    pub fn new(
        settlement_amount: f64,
        currency: String,
        creditor_name: String,
        debtor_name: String,
        description: String,
    ) -> Self {
        Self {
            msg_id: Self::generate_message_id(),
            creation_date_time: Utc::now(),
            settlement_amount,
            currency,
            creditor_name,
            debtor_name,
            description,
            payment_method: "PIVA".to_string(),
        }
    }
    
    /// Generate unique message ID
    fn generate_message_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        Utc::now().timestamp().hash(&mut hasher);
        format!("PIVA-{:x}", hasher.finish())
    }
    
    /// Convert to XML (pain.001 format)
    pub fn to_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.03">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>{msg_id}</MsgId>
      <CreDtTm>{creation_time}</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{amount:.2}</CtrlSum>
      <InitgPty>
        <Nm>PIVA Protocol</Nm>
      </InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-{msg_id}</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{amount:.2}</CtrlSum>
      <ReqdExctnDt>{execution_date}</ReqdExctnDt>
      <Dbtr>
        <Nm>{debtor_name}</Nm>
      </Dbtr>
      <Cdtr>
        <Nm>{creditor_name}</Nm>
      </Cdtr>
      <CdtTrfTxInf>
        <PmtId>
          <InstrId>INSTR-{msg_id}</InstrId>
          <EndToEndId>E2E-{msg_id}</EndToEndId>
        </PmtId>
        <Amt>
          <InstdAmt Ccy="{currency}">{amount:.2}</InstdAmt>
        </Amt>
        <RmtInf>
          <Ustrd>{description}</Ustrd>
        </RmtInf>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#,
            msg_id = self.msg_id,
            creation_time = self.creation_date_time.format("%Y-%m-%dT%H:%M:%S"),
            amount = self.settlement_amount,
            currency = self.currency,
            debtor_name = self.debtor_name,
            creditor_name = self.creditor_name,
            execution_date = self.creation_date_time.format("%Y-%m-%d"),
            description = self.description
        )
    }
    
    /// Validate XML structure
    pub fn validate_xml(&self) -> bool {
        let xml = self.to_xml();
        xml.contains("CstmrCdtTrfInitn") && 
        xml.contains("GrpHdr") && 
        xml.contains("PmtInf") &&
        xml.contains(&self.msg_id) &&
        xml.contains(&format!("{:.2}", self.settlement_amount))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IsoMessageType {
    /// Customer Credit Transfer (pain.001)
    CustomerCreditTransfer,
    /// Customer Payment Status (pain.002)
    CustomerPaymentStatus,
    /// Financial Institution Transfer (pacs.008)
    FinancialInstitutionTransfer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferStatus {
    /// Status code
    pub code: String,
    
    /// Status description
    pub description: String,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl TransferStatus {
    /// Create new transfer status
    pub fn new(code: String, description: String) -> Self {
        Self {
            code,
            description,
            timestamp: Utc::now(),
        }
    }
    
    /// Common status codes
    pub const ACCEPTED: &'static str = "ACCP";
    pub const REJECTED: &'static str = "RJCT";
    pub const PENDING: &'static str = "PDNG";
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_iso_report_creation() {
        let report = Iso20022Report::new(
            1000.50,
            "USD".to_string(),
            "Juan Perez".to_string(),
            "Maria Garcia".to_string(),
            "PIVA Asset Transfer: test_123".to_string(),
        );
        
        assert_eq!(report.currency, "USD");
        assert_eq!(report.settlement_amount, 1000.50);
        assert_eq!(report.creditor_name, "Juan Perez");
        assert_eq!(report.debtor_name, "Maria Garcia");
        assert!(report.msg_id.starts_with("PIVA-"));
    }
    
    #[test]
    fn test_xml_generation() {
        let report = Iso20022Report::new(
            500.00,
            "MXN".to_string(),
            "Carlos Rodriguez".to_string(),
            "Ana Martinez".to_string(),
            "PIVA Voucher Purchase".to_string(),
        );
        
        let xml = report.to_xml();
        
        assert!(xml.contains("CstmrCdtTrfInitn"));
        assert!(xml.contains("500.00"));
        assert!(xml.contains("MXN"));
        assert!(xml.contains("Carlos Rodriguez"));
        assert!(xml.contains("Ana Martinez"));
        assert!(xml.contains("PIVA Voucher Purchase"));
        assert!(report.validate_xml());
    }
    
    #[test]
    fn test_transfer_status() {
        let status = TransferStatus::new(
            TransferStatus::ACCEPTED.to_string(),
            "Payment accepted".to_string(),
        );
        
        assert_eq!(status.code, TransferStatus::ACCEPTED);
        assert_eq!(status.description, "Payment accepted");
    }
}
