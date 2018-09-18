#![cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]

use std::str::FromStr;

use symbolic::debuginfo::DebugId;
use uuid::Uuid;

pub fn validate_org(v: String) -> Result<(), String> {
    if v.contains('/') || v == "." || v == ".." || v.contains(' ') {
        Err("invalid value for organization. Use the URL slug and not the name!".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_project(v: String) -> Result<(), String> {
    if v.contains('/')
        || v == "."
        || v == ".."
        || v.contains(' ')
        || v.contains('\n')
        || v.contains('\t')
        || v.contains('\r')
    {
        Err("invalid value for project. Use the URL slug and not the name!".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_uuid(s: String) -> Result<(), String> {
    if Uuid::parse_str(&s).is_err() {
        Err("Invalid UUID".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_id(s: String) -> Result<(), String> {
    if DebugId::from_str(&s).is_err() {
        Err("Invalid ID".to_string())
    } else {
        Ok(())
    }
}
