//! # ISO 20022 Adapter
//! 
//! Mapping between PIVA internal states and ISO 20022 financial messages.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IsoError {
    #[error("XML generation error: {0}")]
    XmlError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Template error: {0}")]
    TemplateError(String),
}

/// Generic ISO 20022 message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsoMessage {
    Pain001(Pain001Message),
    Pacs008(Pacs008Message),
    Camt054(Camt054Message),
}

/// Pain.001 - Customer Credit Transfer Initiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pain001Message {
    pub msg_id: String,
    pub cre_dt_tm: String, // ISO 8601 timestamp
    pub nb_of_txs: String,
    pub ctrl_sum: Option<String>,
    pub initg_pty: Party,
    pub dbtr: Party,
    pub dbtr_acct: Account,
    pub dbtr_agt: Agent,
    pub cdtr_agt: Agent,
    pub cdtr: Party,
    pub cdtr_acct: Account,
    pub amt: Amount,
    pub purp: Option<PaymentPurpose>,
}

/// Pacs.008 - Financial Institution Transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacs008Message {
    pub tx_id: String,
    pub intr_bk_sttlm_dt: String,
    pub intr_bk_sttlm_amt: Amount,
    pub intr_bk_sttlm_acct: Account,
    pub cdtr_agt: Agent,
    pub cdtr: Party,
    pub cdtr_acct: Account,
    pub rmt_inf: Option<RemittanceInfo>,
}

/// Camt.054 - Bank To Customer Account Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt054Message {
    pub msg_id: String,
    pub cre_dt_tm: String,
    pub acct: Account,
    pub bal: Vec<Balance>,
    pub tx: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub nm: Option<String>,
    pub pstl_adr: Option<PostalAddress>,
    pub id: Option<PartyId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalAddress {
    pub strt_nm: Option<String>,
    pub pst_cd: Option<String>,
    pub twn_nm: Option<String>,
    pub ctry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyId {
    pub org_id: Option<String>,
    pub prvt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub iban: Option<String>,
    pub cur: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub fin_instn_id: String,
    pub bic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub value: String,
    pub cur: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPurpose {
    pub cd: Option<String>,
    pub cdprty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceInfo {
    pub ustrd: Option<String>,
    pub addtl_ri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub tp: BalanceType,
    pub amt: Amount,
    pub dt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceType {
    pub cd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_id: String,
    pub amt: Amount,
    pub cdtr_agt: Option<Agent>,
    pub cdtr: Option<Party>,
    pub dbtr_agt: Option<Agent>,
    pub dbtr: Option<Party>,
    pub rmt_inf: Option<RemittanceInfo>,
}

/// Trait for converting PIVA entities to ISO 20022 messages
pub trait ToIso20022 {
    fn to_pain001(&self) -> Result<String, IsoError>;
    fn to_pacs008(&self) -> Result<String, IsoError>;
    fn to_camt054(&self) -> Result<String, IsoError>;
}

/// Implementation for transfer receipts (placeholder)
impl ToIso20022 for TransferReceipt {
    fn to_pain001(&self) -> Result<String, IsoError> {
        let message = Pain001Message {
            msg_id: format!("PIVA-{}", self.transfer_id),
            cre_dt_tm: chrono::Utc::now().to_rfc3339(),
            nb_of_txs: "1".to_string(),
            ctrl_sum: None,
            initg_pty: Party {
                nm: Some("PIVA Protocol".to_string()),
                pstl_adr: None,
                id: None,
            },
            dbtr: Party {
                nm: Some(self.from_party.clone()),
                pstl_adr: None,
                id: None,
            },
            dbtr_acct: Account {
                id: self.from_account.clone(),
                iban: None,
                cur: self.currency.clone(),
            },
            dbtr_agt: Agent {
                fin_instn_id: self.from_agent.clone(),
                bic: None,
            },
            cdtr_agt: Agent {
                fin_instn_id: self.to_agent.clone(),
                bic: None,
            },
            cdtr: Party {
                nm: Some(self.to_party.clone()),
                pstl_adr: None,
                id: None,
            },
            cdtr_acct: Account {
                id: self.to_account.clone(),
                iban: None,
                cur: self.currency.clone(),
            },
            amt: Amount {
                value: self.amount.to_string(),
                cur: self.currency.clone(),
            },
            purp: Some(PaymentPurpose {
                cd: Some("RWA".to_string()),
                cdprty: Some("PIVA".to_string()),
            }),
        };
        
        generate_pain001_xml(&message)
    }
    
    fn to_pacs008(&self) -> Result<String, IsoError> {
        let message = Pacs008Message {
            tx_id: format!("PIVA-{}", self.transfer_id),
            intr_bk_sttlm_dt: chrono::Utc::now().to_rfc3339(),
            intr_bk_sttlm_amt: Amount {
                value: self.amount.to_string(),
                cur: self.currency.clone(),
            },
            intr_bk_sttlm_acct: Account {
                id: "SETTLEMENT".to_string(),
                iban: None,
                cur: self.currency.clone(),
            },
            cdtr_agt: Agent {
                fin_instn_id: self.to_agent.clone(),
                bic: None,
            },
            cdtr: Party {
                nm: Some(self.to_party.clone()),
                pstl_adr: None,
                id: None,
            },
            cdtr_acct: Account {
                id: self.to_account.clone(),
                iban: None,
                cur: self.currency.clone(),
            },
            rmt_inf: Some(RemittanceInfo {
                ustrd: Some(format!("PIVA RWA Transfer: {}", self.asset_id)),
                addtl_ri: None,
            }),
        };
        
        generate_pacs008_xml(&message)
    }
    
    fn to_camt054(&self) -> Result<String, IsoError> {
        let message = Camt054Message {
            msg_id: format!("PIVA-CAMT-{}", self.transfer_id),
            cre_dt_tm: chrono::Utc::now().to_rfc3339(),
            acct: Account {
                id: self.from_account.clone(),
                iban: None,
                cur: self.currency.clone(),
            },
            bal: vec![Balance {
                tp: BalanceType {
                    cd: "PRCD".to_string(), // Previously booked
                },
                amt: Amount {
                    value: self.amount.to_string(),
                    cur: self.currency.clone(),
                },
                dt: Some(chrono::Utc::now().to_rfc3339()),
            }],
            tx: vec![Transaction {
                tx_id: format!("PIVA-{}", self.transfer_id),
                amt: Amount {
                    value: self.amount.to_string(),
                    cur: self.currency.clone(),
                },
                cdtr_agt: Some(Agent {
                    fin_instn_id: self.to_agent.clone(),
                    bic: None,
                }),
                cdtr: Some(Party {
                    nm: Some(self.to_party.clone()),
                    pstl_adr: None,
                    id: None,
                }),
                dbtr_agt: Some(Agent {
                    fin_instn_id: self.from_agent.clone(),
                    bic: None,
                }),
                dbtr: Some(Party {
                    nm: Some(self.from_party.clone()),
                    pstl_adr: None,
                    id: None,
                }),
                rmt_inf: Some(RemittanceInfo {
                    ustrd: Some(format!("PIVA RWA Transfer: {}", self.asset_id)),
                    addtl_ri: None,
                }),
            }],
        };
        
        generate_camt054_xml(&message)
    }
}

/// Placeholder transfer receipt structure
#[derive(Debug, Clone)]
pub struct TransferReceipt {
    pub transfer_id: String,
    pub asset_id: String,
    pub from_party: String,
    pub from_account: String,
    pub from_agent: String,
    pub to_party: String,
    pub to_account: String,
    pub to_agent: String,
    pub amount: f64,
    pub currency: String,
}

/// Generate Pain.001 XML (simplified)
fn generate_pain001_xml(message: &Pain001Message) -> Result<String, IsoError> {
    // TODO: Implement proper XML generation with quick-xml
    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.03">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>{}</MsgId>
      <CreDtTm>{}</CreDtTm>
      <NbOfTxs>{}</NbOfTxs>
    </GrpHdr>
    <PmtInf>
      <Dbtr>
        <Nm>{}</Nm>
      </Dbtr>
      <DbtrAcct>
        <Id>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </Id>
      </DbtrAcct>
      <CdtrAgt>
        <FinInstnId>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{}</Nm>
      </Cdtr>
      <CdtrAcct>
        <Id>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </Id>
      </CdtrAcct>
      <Amt>
        <InstdAmt Ccy="{}">{}</InstdAmt>
      </Amt>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#,
        message.msg_id,
        message.cre_dt_tm,
        message.nb_of_txs,
        message.dbtr.nm.as_deref().unwrap_or(""),
        message.dbtr_acct.id,
        message.cdtr_agt.fin_instn_id,
        message.cdtr.nm.as_deref().unwrap_or(""),
        message.cdtr_acct.id,
        message.amt.cur,
        message.amt.value
    );
    
    Ok(xml)
}

/// Generate Pacs.008 XML (simplified)
fn generate_pacs008_xml(message: &Pacs008Message) -> Result<String, IsoError> {
    // TODO: Implement proper XML generation with quick-xml
    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.02">
  <FIToFICstmrCdtTrf>
    <Hdr>
      <MsgId>{}</MsgId>
    </Hdr>
    <CdtTrfTxInf>
      <IntrBkSttlmDt>{}</IntrBkSttlmDt>
      <IntrBkSttlmAmt Ccy="{}">{}</IntrBkSttlmAmt>
      <CdtrAgt>
        <FinInstnId>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{}</Nm>
      </Cdtr>
      <CdtrAcct>
        <Id>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </Id>
      </CdtrAcct>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#,
        message.tx_id,
        message.intr_bk_sttlm_dt,
        message.intr_bk_sttlm_amt.cur,
        message.intr_bk_sttlm_amt.value,
        message.cdtr_agt.fin_instn_id,
        message.cdtr.nm.as_deref().unwrap_or(""),
        message.cdtr_acct.id
    );
    
    Ok(xml)
}

/// Generate Camt.054 XML (simplified)
fn generate_camt054_xml(message: &Camt054Message) -> Result<String, IsoError> {
    // TODO: Implement proper XML generation with quick-xml
    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.054.001.04">
  <BkToCstmrAcctRpt>
    <GrpHdr>
      <MsgId>{}</MsgId>
      <CreDtTm>{}</CreDtTm>
    </GrpHdr>
    <Rpt>
      <Acct>
        <Id>
          <Othr>
            <Id>{}</Id>
          </Othr>
        </Id>
      </Acct>
      <Bal>
        <Tp>
          <Cd>{}</Cd>
        </Tp>
        <Amt Ccy="{}">{}</Amt>
      </Bal>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#,
        message.msg_id,
        message.cre_dt_tm,
        message.acct.id,
        message.bal[0].tp.cd,
        message.bal[0].amt.cur,
        message.bal[0].amt.value
    );
    
    Ok(xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pain001_generation() {
        let receipt = TransferReceipt {
            transfer_id: "test-123".to_string(),
            asset_id: "piva_dev_abc123".to_string(),
            from_party: "Alice".to_string(),
            from_account: "ACC001".to_string(),
            from_agent: "BANK001".to_string(),
            to_party: "Bob".to_string(),
            to_account: "ACC002".to_string(),
            to_agent: "BANK002".to_string(),
            amount: 1000.0,
            currency: "EUR".to_string(),
        };
        
        let xml = receipt.to_pain001().unwrap();
        assert!(xml.contains("test-123"));
        assert!(xml.contains("Alice"));
        assert!(xml.contains("Bob"));
        assert!(xml.contains("1000"));  // Changed from "1000.0" to match actual format
        assert!(xml.contains("EUR"));
    }
    
    #[test]
    fn test_pacs008_generation() {
        let receipt = TransferReceipt {
            transfer_id: "test-456".to_string(),
            asset_id: "piva_dev_def456".to_string(),
            from_party: "Charlie".to_string(),
            from_account: "ACC003".to_string(),
            from_agent: "BANK003".to_string(),
            to_party: "Diana".to_string(),
            to_account: "ACC004".to_string(),
            to_agent: "BANK004".to_string(),
            amount: 500.0,
            currency: "USD".to_string(),
        };
        
        let xml = receipt.to_pacs008().unwrap();
        assert!(xml.contains("test-456"));
        assert!(xml.contains("Diana"));
        assert!(xml.contains("500"));  // Changed from "500.0" to match actual format
        assert!(xml.contains("USD"));
    }
}
