//! # ISO 20022 XML Templates
//! 
//! Pre-defined templates for generating ISO 20022 messages.

/// Template for Pain.001 Customer Credit Transfer Initiation
pub const PAIN001_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.03">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>{{msg_id}}</MsgId>
      <CreDtTm>{{cre_dt_tm}}</CreDtTm>
      <NbOfTxs>{{nb_of_txs}}</NbOfTxs>
      {{#ctrl_sum}}<CtrlSum>{{ctrl_sum}}</CtrlSum>{{/ctrl_sum}}
    </GrpHdr>
    <PmtInf>
      <PmtInfId>{{pmt_inf_id}}</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <ReqdExctnDt>{{reqd_exctn_dt}}</ReqdExctnDt>
      <Dbtr>
        <Nm>{{dbtr_nm}}</Nm>
        {{#dbtr_pstl_adr}}
        <PstlAdr>
          {{#strt_nm}}<StrtNm>{{strt_nm}}</StrtNm>{{/strt_nm}}
          {{#pst_cd}}<PstCd>{{pst_cd}}</PstCd>{{/pst_cd}}
          {{#twn_nm}}<TwnNm>{{twn_nm}}</TwnNm>{{/twn_nm}}
          <Ctry>{{ctry}}</Ctry>
        </PstlAdr>
        {{/dbtr_pstl_adr}}
      </Dbtr>
      <DbtrAcct>
        <Id>
          {{#dbtr_iban}}
          <IBAN>{{dbtr_iban}}</IBAN>
          {{/dbtr_iban}}
          {{^dbtr_iban}}
          <Othr>
            <Id>{{dbtr_acct_id}}</Id>
          </Othr>
          {{/dbtr_iban}}
        </Id>
      </DbtrAcct>
      <DbtrAgt>
        <FinInstnId>
          {{#dbtr_bic}}
          <BIC>{{dbtr_bic}}</BIC>
          {{/dbtr_bic}}
          {{^dbtr_bic}}
          <Othr>
            <Id>{{dbtr_agent_id}}</Id>
          </Othr>
          {{/dbtr_bic}}
        </FinInstnId>
      </DbtrAgt>
      <CdtrAgt>
        <FinInstnId>
          {{#cdtr_bic}}
          <BIC>{{cdtr_bic}}</BIC>
          {{/cdtr_bic}}
          {{^cdtr_bic}}
          <Othr>
            <Id>{{cdtr_agent_id}}</Id>
          </Othr>
          {{/cdtr_bic}}
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{{cdtr_nm}}</Nm>
        {{#cdtr_pstl_adr}}
        <PstlAdr>
          {{#strt_nm}}<StrtNm>{{strt_nm}}</StrtNm>{{/strt_nm}}
          {{#pst_cd}}<PstCd>{{pst_cd}}</PstCd>{{/pst_cd}}
          {{#twn_nm}}<TwnNm>{{twn_nm}}</TwnNm>{{/twn_nm}}
          <Ctry>{{ctry}}</Ctry>
        </PstlAdr>
        {{/cdtr_pstl_adr}}
      </Cdtr>
      <CdtrAcct>
        <Id>
          {{#cdtr_iban}}
          <IBAN>{{cdtr_iban}}</IBAN>
          {{/cdtr_iban}}
          {{^cdtr_iban}}
          <Othr>
            <Id>{{cdtr_acct_id}}</Id>
          </Othr>
          {{/cdtr_iban}}
        </Id>
      </CdtrAcct>
      <Amt>
        <InstdAmt Ccy="{{currency}}">{{amount}}</InstdAmt>
      </Amt>
      {{#purp_cd}}
      <PmtTpInf>
        <SvcLvl>
          <Cd>SDVA</Cd>
        </SvcLvl>
      </PmtTpInf>
      {{/purp_cd}}
      {{#purp_cd}}
      <PmtTpInf>
        <CtgyPurp>
          <Cd>{{purp_cd}}</Cd>
        </CtgyPurp>
      </PmtTpInf>
      {{/purp_cd}}
      {{#rmt_inf}}
      <RmtInf>
        <Ustrd>{{rmt_inf}}</Ustrd>
      </RmtInf>
      {{/rmt_inf}}
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#;

/// Template for Pacs.008 Financial Institution Transfer
pub const PACS008_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.02">
  <FIToFICstmrCdtTrf>
    <Hdr>
      <MsgId>{{msg_id}}</MsgId>
      <CreDtTm>{{cre_dt_tm}}</CreDtTm>
      <NbOfTxs>{{nb_of_txs}}</NbOfTxs>
    </Hdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>{{instr_id}}</InstrId>
        <EndToEndId>{{end_to_end_id}}</EndToEndId>
        <TxId>{{tx_id}}</TxId>
      </PmtId>
      <Amt>
        <InstdAmt Ccy="{{currency}}">{{amount}}</InstdAmt>
      </Amt>
      <IntrBkSttlmDt>{{intr_bk_sttlm_dt}}</IntrBkSttlmDt>
      <CdtrAgt>
        <FinInstnId>
          {{#cdtr_bic}}
          <BIC>{{cdtr_bic}}</BIC>
          {{/cdtr_bic}}
          {{^cdtr_bic}}
          <Othr>
            <Id>{{cdtr_agent_id}}</Id>
          </Othr>
          {{/cdtr_bic}}
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{{cdtr_nm}}</Nm>
        {{#cdtr_pstl_adr}}
        <PstlAdr>
          {{#strt_nm}}<StrtNm>{{strt_nm}}</StrtNm>{{/strt_nm}}
          {{#pst_cd}}<PstCd>{{pst_cd}}</PstCd>{{/pst_cd}}
          {{#twn_nm}}<TwnNm>{{twn_nm}}</TwnNm>{{/twn_nm}}
          <Ctry>{{ctry}}</Ctry>
        </PstlAdr>
        {{/cdtr_pstl_adr}}
      </Cdtr>
      <CdtrAcct>
        <Id>
          {{#cdtr_iban}}
          <IBAN>{{cdtr_iban}}</IBAN>
          {{/cdtr_iban}}
          {{^cdtr_iban}}
          <Othr>
            <Id>{{cdtr_acct_id}}</Id>
          </Othr>
          {{/cdtr_iban}}
        </Id>
      </CdtrAcct>
      {{#rmt_inf}}
      <RmtInf>
        <Ustrd>{{rmt_inf}}</Ustrd>
      </RmtInf>
      {{/rmt_inf}}
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#;

/// Template for Camt.054 Bank To Customer Account Report
pub const CAMT054_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.054.001.04">
  <BkToCstmrAcctRpt>
    <GrpHdr>
      <MsgId>{{msg_id}}</MsgId>
      <CreDtTm>{{cre_dt_tm}}</CreDtTm>
      <MsgRct>{{msg_rct}}</MsgRct>
    </GrpHdr>
    <Rpt>
      <Id>{{rpt_id}}</Id>
      <ElctrncSeqNb>{{elctrnc_seq_nb}}</ElctrncSeqNb>
      <LglSeqNb>{{lgl_seq_nb}}</LglSeqNb>
      <CreDtTm>{{cre_dt_tm}}</CreDtTm>
      <Acct>
        <Id>
          {{#iban}}
          <IBAN>{{iban}}</IBAN>
          {{/iban}}
          {{^iban}}
          <Othr>
            <Id>{{acct_id}}</Id>
          </Othr>
          {{/iban}}
        </Id>
        <Ccy>{{currency}}</Ccy>
      </Acct>
      {{#balances}}
      <Bal>
        <Tp>
          <Cd>{{type_cd}}</Cd>
          {{#type_prtry}}
          <Prtry>{{type_prtry}}</Prtry>
          {{/type_prtry}}
        </Tp>
        <Amt Ccy="{{currency}}">{{amount}}</Amt>
        {{#dt}}
        <Dt>
          {{#dt}}
          <Dt>{{dt}}</Dt>
          {{/dt}}
          {{#dt_tm}}
          <DtTm>{{dt_tm}}</DtTm>
          {{/dt_tm}}
        </Dt>
        {{/dt}}
      </Bal>
      {{/balances}}
      {{#transactions}}
      <Ntry>
        <Amt Ccy="{{currency}}">{{amount}}</Amt>
        <CdtDbtInd>{{cdt_dbt_ind}}</CdtDbtInd>
        <Sts>BOOK</Sts>
        <BookgDt>
          <Dt>{{booking_date}}</Dt>
        </BookgDt>
        <ValDt>
          <Dt>{{value_date}}</Dt>
        </ValDt>
        <AcctSvcrRef>{{acct_svcr_ref}}</AcctSvcrRef>
        <BkTxCd>
          <Domn>
            <Cd>{{domain_code}}</Cd>
            <Fmly>
              <Cd>{{family_code}}</Cd>
            </Fmly>
          </Domn>
        </BkTxCd>
        <NtryDtls>
          <TxDtls>
            <Refs>
              <EndToEndId>{{end_to_end_id}}</EndToEndId>
            </Refs>
            <Amt>
              <InstdAmt Ccy="{{currency}}">{{amount}}</InstdAmt>
            </Amt>
            {{#cdtr_agt}}
            <CdtrAgt>
              <FinInstnId>
                {{#bic}}
                <BIC>{{bic}}</BIC>
                {{/bic}}
                {{^bic}}
                <Othr>
                  <Id>{{agent_id}}</Id>
                </Othr>
                {{/bic}}
              </FinInstnId>
            </CdtrAgt>
            {{/cdtr_agt}}
            {{#cdtr}}
            <Cdtr>
              <Nm>{{name}}</Nm>
              {{#pstl_adr}}
              <PstlAdr>
                {{#strt_nm}}<StrtNm>{{strt_nm}}</StrtNm>{{/strt_nm}}
                {{#pst_cd}}<PstCd>{{pst_cd}}</PstCd>{{/pst_cd}}
                {{#twn_nm}}<TwnNm>{{twn_nm}}</TwnNm>{{/twn_nm}}
                <Ctry>{{ctry}}</Ctry>
              </PstlAdr>
              {{/pstl_adr}}
            </Cdtr>
            {{/cdtr}}
            {{#dbtr_agt}}
            <DbtrAgt>
              <FinInstnId>
                {{#bic}}
                <BIC>{{bic}}</BIC>
                {{/bic}}
                {{^bic}}
                <Othr>
                  <Id>{{agent_id}}</Id>
                </Othr>
                {{/bic}}
              </FinInstnId>
            </DbtrAgt>
            {{/dbtr_agt}}
            {{#dbtr}}
            <Dbtr>
              <Nm>{{name}}</Nm>
              {{#pstl_adr}}
              <PstlAdr>
                {{#strt_nm}}<StrtNm>{{strt_nm}}</StrtNm>{{/strt_nm}}
                {{#pst_cd}}<PstCd>{{pst_cd}}</PstCd>{{/pst_cd}}
                {{#twn_nm}}<TwnNm>{{twn_nm}}</TwnNm>{{/twn_nm}}
                <Ctry>{{ctry}}</Ctry>
              </PstlAdr>
              {{/pstl_adr}}
            </Dbtr>
            {{/dbtr}}
            {{#rmt_inf}}
            <RmtInf>
              <Ustrd>{{rmt_inf}}</Ustrd>
            </RmtInf>
            {{/rmt_inf}}
          </TxDtls>
        </NtryDtls>
      </Ntry>
      {{/transactions}}
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#;

/// Template data structure for Pain.001
#[derive(Debug, Clone)]
pub struct Pain001Data {
    pub msg_id: String,
    pub cre_dt_tm: String,
    pub nb_of_txs: String,
    pub ctrl_sum: Option<String>,
    pub dbtr_nm: String,
    pub dbtr_pstl_adr: Option<PostalAddressData>,
    pub dbtr_iban: Option<String>,
    pub dbtr_acct_id: Option<String>,
    pub dbtr_bic: Option<String>,
    pub dbtr_agent_id: Option<String>,
    pub cdtr_nm: String,
    pub cdtr_pstl_adr: Option<PostalAddressData>,
    pub cdtr_iban: Option<String>,
    pub cdtr_acct_id: Option<String>,
    pub cdtr_bic: Option<String>,
    pub cdtr_agent_id: Option<String>,
    pub amount: String,
    pub currency: String,
    pub purp_cd: Option<String>,
    pub rmt_inf: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostalAddressData {
    pub strt_nm: Option<String>,
    pub pst_cd: Option<String>,
    pub twn_nm: Option<String>,
    pub ctry: String,
}

/// Render template with data (simplified implementation)
pub fn render_pain001(data: &Pain001Data) -> Result<String, String> {
    // TODO: Implement proper template rendering with handlebars or tera
    let mut template = PAIN001_TEMPLATE.to_string();
    
    // Simple string replacement for now
    template = template.replace("{{msg_id}}", &data.msg_id);
    template = template.replace("{{cre_dt_tm}}", &data.cre_dt_tm);
    template = template.replace("{{nb_of_txs}}", &data.nb_of_txs);
    template = template.replace("{{dbtr_nm}}", &data.dbtr_nm);
    template = template.replace("{{cdtr_nm}}", &data.cdtr_nm);
    template = template.replace("{{amount}}", &data.amount);
    template = template.replace("{{currency}}", &data.currency);
    
    if let Some(rmt_inf) = &data.rmt_inf {
        template = template.replace("{{rmt_inf}}", rmt_inf);
    } else {
        template = template.replace("{{#rmt_inf}}", "")
                           .replace("{{/rmt_inf}}", "");
    }
    
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pain001_template_render() {
        let data = Pain001Data {
            msg_id: "TEST-001".to_string(),
            cre_dt_tm: "2024-01-01T12:00:00Z".to_string(),
            nb_of_txs: "1".to_string(),
            ctrl_sum: None,
            dbtr_nm: "Alice".to_string(),
            dbtr_pstl_adr: None,
            dbtr_iban: Some("AL89213110090000000012345678".to_string()),
            dbtr_acct_id: None,
            dbtr_bic: Some("BEXPALAA".to_string()),
            dbtr_agent_id: None,
            cdtr_nm: "Bob".to_string(),
            cdtr_pstl_adr: None,
            cdtr_iban: Some("GB82WEST12345698765432".to_string()),
            cdtr_acct_id: None,
            cdtr_bic: Some("BARCGB22".to_string()),
            cdtr_agent_id: None,
            amount: "1000.00".to_string(),
            currency: "EUR".to_string(),
            purp_cd: Some("RWA".to_string()),
            rmt_inf: Some("PIVA RWA Transfer".to_string()),
        };
        
        let xml = render_pain001(&data).unwrap();
        assert!(xml.contains("TEST-001"));
        assert!(xml.contains("Alice"));
        assert!(xml.contains("Bob"));
        assert!(xml.contains("1000.00"));
        assert!(xml.contains("EUR"));
        assert!(xml.contains("PIVA RWA Transfer"));
    }
}
