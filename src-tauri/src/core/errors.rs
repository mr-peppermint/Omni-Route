use std::fmt;

/// Errore comune per il layer di rete/discovery. Distingue esplicitamente
/// i fallimenti Win32 (codice + nome funzione) dagli errori io/wmi generici
/// perché il messaggio deve restare diagnosticabile da `omni-route doctor`.
#[derive(Debug)]
pub enum OmniRouteError {
    Win32 { function: &'static str, code: u32 },
    Io(std::io::Error),
    Wmi(String),
    /// `WlanQueryInterface`/`netsh wlan` falliscono con `ERROR_ACCESS_DENIED`
    /// se i Servizi di localizzazione di Windows sono disattivati: da
    /// Windows 10 leggere SSID/BSSID richiede il permesso di posizione
    /// (SSID/BSSID sono usabili per geolocalizzazione). Confermato
    /// riproducendo lo stesso errore con `netsh wlan show interfaces`.
    LocationServicesRequired,
    /// Errori di protocollo/logica HTTP: URL non valido, status inatteso
    /// durante probe/download, troppi redirect, risposta incoerente con
    /// quanto atteso da §4.6/§4.7 della spec.
    Http(String),
    Sqlite(String),
    /// Fallimento di trasporto (connessione, handshake, invio/ricezione a
    /// livello TCP): a differenza di `Http`, che segnala una decisione di
    /// protocollo (status inatteso, ETag divergente — mai da riprovare
    /// ciecamente), questi sono transitori per natura e il chiamante può
    /// ritentare (§4.7 della spec: "Timeout / connection reset → segmento
    /// in coda").
    Transport(String),
    /// Download interrotto dall'utente (pulsante Annulla, §6.2 della UI):
    /// non è un fallimento, va mostrato/loggato diversamente da un errore
    /// vero.
    Cancelled,
}

impl fmt::Display for OmniRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OmniRouteError::Win32 { function, code } => {
                write!(f, "{function} failed (WIN32_ERROR {code})")
            }
            OmniRouteError::Io(e) => write!(f, "IO error: {e}"),
            OmniRouteError::Wmi(msg) => write!(f, "WMI error: {msg}"),
            OmniRouteError::LocationServicesRequired => write!(
                f,
                "SSID/signal unreadable: enable Location Services in \
                 Settings → Privacy & Security → Location (required by Windows \
                 to read Wi-Fi status)"
            ),
            OmniRouteError::Http(msg) => write!(f, "{msg}"),
            OmniRouteError::Sqlite(msg) => write!(f, "database error: {msg}"),
            OmniRouteError::Transport(msg) => write!(f, "transport error: {msg}"),
            OmniRouteError::Cancelled => write!(f, "cancelled by user"),
        }
    }
}

impl std::error::Error for OmniRouteError {}

impl From<std::io::Error> for OmniRouteError {
    fn from(e: std::io::Error) -> Self {
        OmniRouteError::Io(e)
    }
}

impl From<rusqlite::Error> for OmniRouteError {
    fn from(e: rusqlite::Error) -> Self {
        OmniRouteError::Sqlite(e.to_string())
    }
}

pub fn win32_err(function: &'static str, code: u32) -> OmniRouteError {
    OmniRouteError::Win32 { function, code }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win32_error_message_includes_function_name_and_code() {
        let e = win32_err("GetAdaptersAddresses", 111);
        assert_eq!(
            e.to_string(),
            "GetAdaptersAddresses failed (WIN32_ERROR 111)"
        );
    }

    #[test]
    fn location_services_required_message_mentions_settings_path() {
        let e = OmniRouteError::LocationServicesRequired;
        assert!(e.to_string().contains("Location Services"));
        assert!(e.to_string().contains("Location"));
    }
}
