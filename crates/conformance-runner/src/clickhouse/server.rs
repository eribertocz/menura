//! A `ClickHouse` server reached over its HTTP interface, and the sessions the runner opens on
//! it: one fresh session per case, with the settings of E6-7.8 set to their opposite values
//! (E6-7.10).

use super::http::{Endpoint, post};
use super::settings::REVERSED_SETTINGS;

/// How many seconds an idle session lives on the server.
const SESSION_TIMEOUT_SECONDS: &str = "600";

/// Why a request to the server did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerFailure {
    /// The request could not be sent or its response could not be read.
    Transport(String),
    /// The server answered with an error, with its message.
    Engine(String),
}

impl ServerFailure {
    /// The failure as one line of text.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Transport(reason) => reason.clone(),
            Self::Engine(message) => format!("ClickHouse returns the error: {}", message.trim()),
        }
    }
}

/// A `ClickHouse` server.
#[derive(Debug, Clone)]
pub struct ClickhouseServer {
    endpoint: Endpoint,
}

impl ClickhouseServer {
    /// The server at `url`, `http://host:port`.
    ///
    /// # Errors
    ///
    /// Why `url` is not such an endpoint.
    pub fn new(url: &str) -> Result<Self, String> {
        Endpoint::parse(url).map(|endpoint| Self { endpoint })
    }

    /// The text the server returns for `statement` outside any session, in `TabSeparated`
    /// form.
    pub(crate) fn query(&self, statement: &str) -> Result<String, ServerFailure> {
        self.send(&[], statement.as_bytes())
    }

    /// Opens the fresh session `id` with the opposite settings of E6-7.10.
    pub(crate) fn open_session(&self, id: String) -> Result<Session<'_>, ServerFailure> {
        let session = Session { server: self, id };
        for setting in REVERSED_SETTINGS {
            session.execute(&format!("SET {} = {}", setting.name, setting.session), &[])?;
        }
        Ok(session)
    }

    /// Sends one request whose URL holds `query` and whose body is `body`. The server is asked
    /// to buffer the whole result, so that an error raised while the rows are computed is an
    /// error status, never a truncated body.
    fn send(&self, query: &[(&str, &str)], body: &[u8]) -> Result<String, ServerFailure> {
        let mut pairs = vec![
            ("wait_end_of_query", "1"),
            ("default_format", "TabSeparated"),
        ];
        pairs.extend_from_slice(query);
        let response = post(&self.endpoint, &pairs, body).map_err(ServerFailure::Transport)?;
        let text = String::from_utf8(response.body)
            .map_err(|_| ServerFailure::Transport("the response is not UTF-8".to_owned()))?;
        if response.status == 200 {
            Ok(text)
        } else {
            Err(ServerFailure::Engine(text))
        }
    }
}

/// A session of a [`ClickhouseServer`]: every statement of one case runs in it.
#[derive(Debug)]
pub(crate) struct Session<'server> {
    server: &'server ClickhouseServer,
    id: String,
}

impl Session<'_> {
    /// The `TabSeparated` text `statement` returns, with `parameters` bound as the query
    /// parameters `param_<name>` (E6-7.3).
    pub(crate) fn execute(
        &self,
        statement: &str,
        parameters: &[(String, String)],
    ) -> Result<String, ServerFailure> {
        let names: Vec<String> = parameters
            .iter()
            .map(|(name, _)| format!("param_{name}"))
            .collect();
        let mut query = self.session_pairs();
        query.extend(
            names
                .iter()
                .zip(parameters)
                .map(|(name, (_, value))| (name.as_str(), value.as_str())),
        );
        self.server.send(&query, statement.as_bytes())
    }

    /// Runs `statement`, an `INSERT … FORMAT TabSeparated`, with `data` as its rows.
    pub(crate) fn insert(&self, statement: &str, data: &[u8]) -> Result<(), ServerFailure> {
        let mut query = self.session_pairs();
        query.push(("query", statement));
        self.server.send(&query, data).map(|_| ())
    }

    fn session_pairs(&self) -> Vec<(&str, &str)> {
        vec![
            ("session_id", self.id.as_str()),
            ("session_timeout", SESSION_TIMEOUT_SECONDS),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e6_7_10_an_engine_failure_names_clickhouse() {
        let failure = ServerFailure::Engine("Code: 395. boom\n".to_owned());
        assert_eq!(
            failure.describe(),
            "ClickHouse returns the error: Code: 395. boom"
        );
    }
}
