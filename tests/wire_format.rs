//! Serialized data must stay readable across versions.
//!
//! The documents below were written by mirifici 0.1. Each test checks both directions:
//! today's code reads the frozen document to the same values, and writes the same values
//! to the same document. If a test here fails, a change has broken stored data: do not
//! update the document, restore compatibility (or, for a deliberate break, bump the major
//! version and say so in the changelog).
#![cfg(feature = "serde")]

use mirifici::{Log, SignedLog};
use serde_test::{Configure, Token, assert_tokens};

type L = Log<f64>;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Record {
    stored: Vec<L>,
    stored_f32: Vec<Log<f32>>,
    signed: Vec<SignedLog>,
    #[serde(with = "mirifici::serde_as_value")]
    value_small: L,
    #[serde(with = "mirifici::serde_as_value")]
    value_tiny: L,
    #[serde(with = "mirifici::serde_as_value")]
    value_zero: L,
    #[serde(with = "mirifici::serde_as_value")]
    value_inf: L,
}

/// Frozen at 0.1.0.
const RECORD_V0_1: &str = concat!(
    r#"{"stored":["-inf",0.0,"inf",-1.3862943611198906,-11512.925464970229],"#,
    r#""stored_f32":["-inf",0.0,-2.3025851],"#,
    r#""signed":[[false,"-inf"],[true,0.0],[false,"inf"],[true,-11512.925464970229]],"#,
    r#""value_small":0.25,"value_tiny":"1e-5000","value_zero":0.0,"value_inf":"inf"}"#
);

fn record() -> Record {
    let tiny = L::from_log10(-5000.0);
    Record {
        stored: vec![L::ZERO, L::ONE, L::INFINITY, L::from_value(0.25), tiny],
        stored_f32: vec![Log::ZERO, Log::ONE, Log::from_value(0.1_f32)],
        signed: vec![
            SignedLog::ZERO,
            SignedLog::NEG_ONE,
            SignedLog::INFINITY,
            SignedLog::new(true, tiny),
        ],
        value_small: L::from_value(0.25),
        value_tiny: tiny,
        value_zero: L::ZERO,
        value_inf: L::INFINITY,
    }
}

#[test]
fn json_reads_frozen_document() {
    let read: Record = serde_json::from_str(RECORD_V0_1).unwrap();
    assert_eq!(read, record());
}

#[test]
fn json_writes_frozen_document() {
    assert_eq!(serde_json::to_string(&record()).unwrap(), RECORD_V0_1);
}

#[test]
fn compact_formats_store_the_raw_float() {
    // Binary formats (bincode, postcard, …) see exactly one float per Log.
    assert_tokens(&L::from_ln(-2.5).compact(), &[Token::F64(-2.5)]);
    assert_tokens(&L::ZERO.compact(), &[Token::F64(f64::NEG_INFINITY)]);
    assert_tokens(&Log::<f32>::from_ln(-2.5).compact(), &[Token::F32(-2.5)]);
    assert_tokens(
        &SignedLog::new(true, L::from_ln(-2.5)).compact(),
        &[
            Token::Tuple { len: 2 },
            Token::Bool(true),
            Token::F64(-2.5),
            Token::TupleEnd,
        ],
    );
}
