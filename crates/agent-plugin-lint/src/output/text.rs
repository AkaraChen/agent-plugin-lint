use crate::{CoverageStatus, Radius, Report, Scope};

pub fn render(report: &Report) -> String {
    TextReport::project(report).write()
}

struct TextReport<'a> {
    records: Vec<Record<'a>>,
}

enum Record<'a> {
    Error(ErrorRecord<'a>),
    Plugin(PluginRecord<'a>),
    Finding(FindingRecord<'a>),
    Coverage(CoverageRecord<'a>),
    Summary(SummaryRecord),
}

struct ErrorRecord<'a> {
    code: &'a str,
    path: &'a str,
    message: &'a str,
}

struct PluginRecord<'a> {
    root: &'a str,
    status: &'a str,
}

struct FindingRecord<'a> {
    rule: &'a str,
    path: &'a str,
    spec: String,
    radius: &'static str,
    effect: &'static str,
    scope: String,
    evidence: &'a str,
    message: &'a str,
    pointer: Option<&'a str>,
    hint: Option<&'a str>,
    line: Option<usize>,
    column: Option<usize>,
}

struct CoverageRecord<'a> {
    rule: &'a str,
    target: &'a str,
    status: &'static str,
    reason: Option<&'a str>,
}

struct SummaryRecord {
    fatal: usize,
    component: usize,
    ignored: usize,
    advisory: usize,
    errors: usize,
    exit_code: i32,
}

impl<'a> TextReport<'a> {
    fn project(report: &'a Report) -> Self {
        let mut records = Vec::new();
        for error in &report.errors {
            records.push(Record::Error(ErrorRecord {
                code: &error.code,
                path: &error.path,
                message: &error.message,
            }));
        }
        for plugin in &report.plugins {
            records.push(Record::Plugin(PluginRecord {
                root: &plugin.root,
                status: &plugin.status,
            }));
            for finding in &plugin.findings {
                records.push(Record::Finding(FindingRecord {
                    rule: finding.rule_id.as_str(),
                    path: &finding.path,
                    spec: finding.spec.join(", "),
                    radius: radius_label(finding.radius),
                    effect: finding.effect.as_str(),
                    scope: scope_label(&finding.scope),
                    evidence: &finding.evidence_code,
                    message: &finding.message,
                    pointer: finding.pointer.as_deref(),
                    hint: finding.hint.as_deref(),
                    line: finding.line,
                    column: finding.column,
                }));
            }
            for coverage in &plugin.coverage {
                if matches!(
                    coverage.status,
                    CoverageStatus::Pass | CoverageStatus::NotApplicable
                ) {
                    continue;
                }
                records.push(Record::Coverage(CoverageRecord {
                    rule: &coverage.rule_id,
                    target: &coverage.target,
                    status: coverage.status.as_str(),
                    reason: coverage.reason_code.as_deref(),
                }));
            }
        }
        records.push(Record::Summary(SummaryRecord {
            fatal: report.summary.fatal,
            component: report.summary.component,
            ignored: report.summary.ignored,
            advisory: report.summary.advisory,
            errors: report.summary.errors,
            exit_code: report.exit_code,
        }));
        Self { records }
    }

    fn write(&self) -> String {
        let mut pipe = Writer::default();
        for record in &self.records {
            match record {
                Record::Error(error) => error.write(&mut pipe),
                Record::Plugin(plugin) => plugin.write(&mut pipe),
                Record::Finding(finding) => finding.write(&mut pipe),
                Record::Coverage(coverage) => coverage.write(&mut pipe),
                Record::Summary(summary) => summary.write(&mut pipe),
            }
        }
        pipe.out
    }
}

impl ErrorRecord<'_> {
    fn write(&self, pipe: &mut Writer) {
        pipe.record("error");
        pipe.field("code", self.code);
        pipe.field("path", self.path);
        pipe.field("message", self.message);
    }
}

impl PluginRecord<'_> {
    fn write(&self, pipe: &mut Writer) {
        pipe.record("plugin");
        pipe.field("root", self.root);
        pipe.field("status", self.status);
    }
}

impl FindingRecord<'_> {
    fn write(&self, pipe: &mut Writer) {
        pipe.record("finding");
        pipe.field("rule", self.rule);
        pipe.field("path", self.path);
        pipe.field("spec", &self.spec);
        pipe.field("radius", self.radius);
        pipe.field("effect", self.effect);
        pipe.field("scope", &self.scope);
        pipe.field("evidence", self.evidence);
        pipe.field("message", self.message);
        pipe.optional("pointer", self.pointer);
        pipe.optional("hint", self.hint);
        if let Some(line) = self.line {
            pipe.field("line", &line.to_string());
        }
        if let Some(column) = self.column {
            pipe.field("column", &column.to_string());
        }
    }
}

impl CoverageRecord<'_> {
    fn write(&self, pipe: &mut Writer) {
        pipe.record("coverage");
        pipe.field("rule", self.rule);
        pipe.field("target", self.target);
        pipe.field("status", self.status);
        pipe.optional("reason", self.reason);
    }
}

impl SummaryRecord {
    fn write(&self, pipe: &mut Writer) {
        pipe.record("summary");
        pipe.field("fatal", &self.fatal.to_string());
        pipe.field("component", &self.component.to_string());
        pipe.field("ignored", &self.ignored.to_string());
        pipe.field("advisory", &self.advisory.to_string());
        pipe.field("errors", &self.errors.to_string());
        pipe.field("exit-code", &self.exit_code.to_string());
    }
}

#[derive(Default)]
struct Writer {
    out: String,
}

impl Writer {
    fn record(&mut self, title: &str) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out.push_str(title);
        self.out.push('\n');
    }

    fn field(&mut self, label: &str, value: &str) {
        self.out.push_str("  ");
        self.out.push_str(label);
        self.out.push_str(": ");
        self.out.push_str(value);
        self.out.push('\n');
    }

    fn optional(&mut self, label: &str, value: Option<&str>) {
        if let Some(value) = value {
            self.field(label, value);
        }
    }
}

fn radius_label(radius: Radius) -> &'static str {
    match radius {
        Radius::Fatal => "fatal",
        Radius::Component => "component",
        Radius::Ignored => "ignored",
        Radius::Advisory => "advisory",
    }
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Plugin => "plugin".to_owned(),
        Scope::ComponentType(id) => format!("component-type {id}"),
        Scope::Skill(id) => format!("skill {id}"),
        Scope::Server(id) => format!("server {id}"),
        Scope::Path(id) => format!("path {id}"),
    }
}
