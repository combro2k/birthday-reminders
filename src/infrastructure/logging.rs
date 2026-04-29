use std::io::Write;
use std::sync::Mutex;

use tracing_subscriber::fmt::MakeWriter;

/// A writer instance that buffers a single log line and sends it to syslog on flush/drop.
pub struct SyslogLineWriter {
    buffer: Vec<u8>,
    logger: std::sync::Arc<Mutex<syslog::Logger<syslog::LoggerBackend, syslog::Formatter3164>>>,
}

impl Write for SyslogLineWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            let msg = String::from_utf8_lossy(&self.buffer).trim_end().to_string();
            if let Ok(mut logger) = self.logger.lock() {
                let _ = logger.info(&msg);
            }
            self.buffer.clear();
        }
        Ok(())
    }
}

impl Drop for SyslogLineWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Wrapper that implements `MakeWriter` for tracing-subscriber.
pub struct SyslogMakeWriter {
    logger: std::sync::Arc<Mutex<syslog::Logger<syslog::LoggerBackend, syslog::Formatter3164>>>,
}

impl SyslogMakeWriter {
    pub fn new() -> anyhow::Result<Self> {
        let formatter = syslog::Formatter3164 {
            facility: syslog::Facility::LOG_DAEMON,
            hostname: None,
            process: "birthday-reminders".to_string(),
            pid: std::process::id(),
        };

        let logger = syslog::unix(formatter)
            .map_err(|e| anyhow::anyhow!("Failed to connect to syslog: {}", e))?;

        Ok(Self {
            logger: std::sync::Arc::new(Mutex::new(logger)),
        })
    }
}

impl<'a> MakeWriter<'a> for SyslogMakeWriter {
    type Writer = SyslogLineWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SyslogLineWriter {
            buffer: Vec::new(),
            logger: self.logger.clone(),
        }
    }
}
