use std::fmt;
use std::fmt::Write;

#[derive(Debug)]
pub enum OpenMetricKind {
    Counter,
    Gauge,
}

impl fmt::Display for OpenMetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenMetricKind::Counter => write!(f, "counter"),
            OpenMetricKind::Gauge => write!(f, "gauge"),
        }
    }
}

#[derive(Debug)]
pub struct OpenMetric<'a> {
    kind: OpenMetricKind,
    name: &'a str,
    help: Option<&'a str>,
    unit: Option<&'a str>,
    timestamp: Option<f64>,
    value: f64,
}

impl<'a> OpenMetric<'a> {
    pub fn new(kind: OpenMetricKind, name: &'a str) -> OpenMetric<'a> {
        OpenMetric {
            kind,
            name,
            help: None,
            unit: None,
            timestamp: None,
            value: 0.,
        }
    }

    pub fn unit(mut self, unit: &'a str) -> OpenMetric<'a> {
        self.unit = Some(unit);
        self
    }

    pub fn help(mut self, help: &'a str) -> OpenMetric<'a> {
        self.help = Some(help);
        self
    }

    pub fn timestamp(mut self, timestamp: f64) -> OpenMetric<'a> {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn value(mut self, value: f64) -> OpenMetric<'a> {
        self.value = value;
        self
    }

    pub fn render(self, out: &mut impl Write) {
        writeln!(out, "# TYPE {} {}", self.name, self.kind).unwrap();
        self.unit.map_or((), |unit| {
            writeln!(out, "# UNIT {} {}", self.name, unit).unwrap()
        });
        self.help.map_or((), |help| {
            writeln!(out, "# HELP {} {}", self.name, help).unwrap()
        });
        match self.timestamp {
            Some(ts) => writeln!(out, "{} {} {}", self.name, self.value, ts * 1000.).unwrap(),
            None => writeln!(out, "{} {}", self.name, self.value).unwrap(),
        };
    }
}
