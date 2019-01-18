use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};

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

    pub fn export(&self, name: &str, report: String) {
        let mut inner = self.0.lock().unwrap();
        inner.reports.insert(name.to_string(), report);

        let mut file = match File::create(&inner.path) {
            Ok(f) => f,
            Err(err) => {
                error!("Error opening export file {}: {}", inner.path, err);
                return;
            }
        };

        if let Err(err) = serde_json::to_writer_pretty(&mut file, &inner.reports) {
            error!("Error exporting report: {}", err);
        }
    }
}
