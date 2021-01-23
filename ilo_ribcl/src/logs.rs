use crate::{
    client, commands,
    ribcl_into::RibclInto,
    types::{NaiveDateTimeBuilder, StringBuilder, U32Builder},
};
use chrono::naive::NaiveDateTime;
use ilo_ribcl_derive::BuilderParse;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct LogEvent {
    pub severity: String,
    pub class: String,
    pub last_update: Option<NaiveDateTime>,
    pub initial_update: Option<NaiveDateTime>,
    pub count: u32,
    pub description: String,
}

impl client::Node {
    /*
    get_method!(
        /// Get the full iLO event log
        rib_info.get_ilo_event_log : "get_event_log" -> "event_log" : Vec<types::LogEvent>
    );
    */
    /// Returns the servers iLO event log
    #[tracing::instrument(skip(self))]
    pub async fn get_ilo_event_log(&mut self) -> Result<Vec<LogEvent>, commands::Error> {
        let mut request = String::new();
        ribcl_command!(request, self.auth(), rib_info, read, get_event_log);
        let response = self.send_ribcl(request.into_bytes()).await?;
        Ok(
            ribcl_parse_response!(response, "event_log" -> Vec<LogEvent>)?.map_err(|source| {
                commands::Error::BuilderParse {
                    target: stringify!($($ret_type)+),
                    source,
                }
            })?,
        )
    }

    mod_method!(
        /// Clears the servers iLO event log
        rib_info.clear_ilo_event_log : "clear_eventlog"
    );

    /// Returns the servers Integrated Management Log (IML).
    // get_method!(server_info.get_server_event_log : "get_event_log" -> "event_log" : Vec<types::LogEvent>);
    #[tracing::instrument(skip(self))]
    pub async fn get_server_event_log(&mut self) -> Result<Vec<LogEvent>, commands::Error> {
        let mut request = String::new();
        ribcl_command!(request, self.auth(), server_info, read, get_event_log);
        let response = self.send_ribcl(request.into_bytes()).await?;
        Ok(
            ribcl_parse_response!(response, "event_log" -> Vec<LogEvent>)?.map_err(|source| {
                commands::Error::BuilderParse {
                    target: stringify!($($ret_type)+),
                    source,
                }
            })?,
        )
    }

    mod_method!(
        /// Clears the servers Integrated Management Log (IML).
        server_info.clear_server_event_log : "clear_iml"
    );
}
