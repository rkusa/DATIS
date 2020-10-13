use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::{error, fmt};

#[derive(Clone)]
pub struct ReportExporter(Arc<Mutex<ReportExporterInner>>);

pub struct ReportExporterInner {
    path: String,
    reports: HashMap<String, String>,
}

impl ReportExporter {
    pub fn new(path: String) -> Self {
        ReportExporter(Arc::new(Mutex::new(ReportExporterInner {
            path,
            reports: HashMap::new(),
        })))
    }

    pub fn export(&self, name: &str, report: String) -> Result<(), ReportExporterError> {
        let mut inner = self.0.lock().unwrap();
        inner.reports.insert(name.to_string(), report);

        let mut file = File::create(&inner.path)?;
        serde_json::to_writer_pretty(&mut file, &inner.reports)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ReportExporterError {
    Io(std::io::Error),
    Json(serde_json::error::Error),
}

impl fmt::Display for ReportExporterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ReportExporterError::*;

        match *self {
            Io(_) => f.write_str("Error opening export file"),
            Json(_) => f.write_str("Error exporting report"),
        }
    }
}

impl error::Error for ReportExporterError {
    fn cause(&self) -> Option<&dyn error::Error> {
        use self::ReportExporterError::*;

        match *self {
            Io(ref err) => Some(err),
            Json(ref err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ReportExporterError {
    fn from(err: std::io::Error) -> Self {
        ReportExporterError::Io(err)
    }
}

impl From<serde_json::Error> for ReportExporterError {
    fn from(err: serde_json::error::Error) -> Self {
        ReportExporterError::Json(err)
    }
}
